//! glab login/status/logout. The token reaches glab on stdin only and is never stored by the app.

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StatusError {
    pub(super) code: &'static str,
    pub(super) message: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ForgeStatus {
    provider: &'static str,
    host: String,
    logged_in: bool,
    login: Option<String>,
    expires_at: Option<String>,
    error: Option<StatusError>,
}

impl ForgeStatus {
    pub(crate) fn github(host: String) -> Self {
        Self {
            provider: "github",
            host,
            logged_in: false,
            login: None,
            expires_at: None,
            error: None,
        }
    }
    fn gitlab(host: &str) -> Self {
        Self {
            provider: "gitlab",
            host: host.into(),
            logged_in: false,
            login: None,
            expires_at: None,
            error: None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Code {
    MissingScope,
    Expired,
    Unauthorized,
    UnreachableOrTls,
    Generic,
}

impl Code {
    fn name(self) -> &'static str {
        match self {
            Code::MissingScope => "missing_scope",
            Code::Expired => "expired",
            Code::Unauthorized => "unauthorized",
            Code::UnreachableOrTls => "unreachable_or_tls",
            Code::Generic => "generic",
        }
    }
    fn message(self, host: &str) -> String {
        match self {
            Code::MissingScope => "Token is missing the `api` scope. Create a new token with `api` and try again.".into(),
            Code::Expired => "This token has expired. Create a new token and try again.".into(),
            Code::Unauthorized => {
                "GitLab rejected the token (401). Check that it was copied in full and has not been revoked.".into()
            }
            Code::UnreachableOrTls => format!(
                "Could not reach {host} or verify its certificate. Check the host and that the internal CA is mounted for the server (`/run/ocr-ca/ca.crt`)."
            ),
            Code::Generic => "GitLab sign-in failed. Check that glab is installed and the personal access token (scope api) is valid and not expired, then reconnect in GitLab connection.".into(),
        }
    }
    fn error(self, host: &str) -> StatusError {
        StatusError {
            code: self.name(),
            message: self.message(host),
        }
    }
}

// shortcut: substrings are assumptions until 02-SPIKE.md records real glab errors, update this table then
const NETWORK_MARKERS: &[&str] = &[
    "x509",
    "certificate",
    "tls:",
    "no such host",
    "connection refused",
    "i/o timeout",
    "dial tcp",
];

pub(super) fn classify(text: &str, host: &str) -> StatusError {
    let lower = text.to_lowercase();
    let code = if NETWORK_MARKERS.iter().any(|m| lower.contains(m)) {
        Code::UnreachableOrTls
    } else if lower.contains("401") && lower.contains("expired") {
        Code::Expired
    } else if lower.contains("401") {
        Code::Unauthorized
    } else {
        Code::Generic
    };
    code.error(host)
}

fn token_problem(json: &serde_json::Value) -> Option<Code> {
    if json["revoked"].as_bool() == Some(true) || json["active"].as_bool() == Some(false) {
        return Some(Code::Expired);
    }
    if let Some(date) = json["expires_at"].as_str() {
        let end = crate::github::iso8601_to_epoch_ms(&format!("{date}T23:59:59Z"));
        if end.is_some_and(|end| end < crate::db::now_millis()) {
            return Some(Code::Expired);
        }
    }
    let scopes = json["scopes"].as_array()?;
    (!scopes.iter().any(|s| s.as_str() == Some("api"))).then_some(Code::MissingScope)
}

pub(crate) fn redact(text: &str, token: &str) -> String {
    let mut out = if token.is_empty() {
        text.to_string()
    } else {
        text.replace(token, "[redacted]")
    };
    while let Some(start) = out.find("glpat-") {
        let run = out[start + 6..]
            .bytes()
            .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_' || *b == b'-')
            .count();
        if run < 10 {
            // Not a token: break the marker so the scan makes progress.
            out.replace_range(start..start + 6, "glpat\u{0}");
            continue;
        }
        out.replace_range(start..start + 6 + run, "[redacted]");
    }
    out.replace("glpat\u{0}", "glpat-")
}

pub(crate) fn parse_token_body(body: &[u8]) -> Result<String, String> {
    #[derive(Deserialize)]
    struct TokenBody {
        token: String,
    }
    let parsed: TokenBody = serde_json::from_slice(body).map_err(|_| "Invalid request body.".to_string())?;
    let token = parsed.token.trim();
    if token.is_empty() {
        return Err("Paste a personal access token.".into());
    }
    if token.len() > 512 || !token.bytes().all(|b| b.is_ascii_graphic()) {
        return Err("That does not look like a personal access token.".into());
    }
    Ok(token.to_string())
}

const SHORT: Duration = Duration::from_secs(20);

pub(super) fn config_dir() -> PathBuf {
    let dir = PathBuf::from(std::env::var("OCR_DATA_DIR").unwrap_or_else(|_| "data".into())).join("glab");
    let _ = crate::github::create_private_dir(&dir);
    dir
}

fn apply_glab_env(cmd: &mut Command, host: &str, config_dir: &Path) {
    cmd.env("GLAB_CONFIG_DIR", config_dir)
        .env("GITLAB_HOST", host)
        .env("GLAB_SEND_TELEMETRY", "false")
        .env("GLAB_CHECK_UPDATE", "false")
        .env("NO_PROMPT", "1")
        .env_remove("GITLAB_TOKEN")
        .env_remove("GITLAB_ACCESS_TOKEN")
        .env_remove("OAUTH_TOKEN")
        .env_remove("GLAB_TOKEN");
}

pub(super) fn run_glab(
    bin: &Path,
    host: &str,
    dir: &Path,
    args: &[&str],
    stdin: Option<&str>,
    timeout: Duration,
) -> Result<Output, String> {
    run_glab_cancellable(bin, host, dir, args, stdin, timeout, &AtomicBool::new(false))
}

pub(super) fn run_glab_cancellable(
    bin: &Path,
    host: &str,
    dir: &Path,
    args: &[&str],
    stdin: Option<&str>,
    timeout: Duration,
    cancel: &AtomicBool,
) -> Result<Output, String> {
    let mut cmd = Command::new(bin);
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    apply_glab_env(&mut cmd, host, dir);
    let mut child = cmd.spawn().map_err(|_| "glab is not available".to_string())?;
    if let Some(mut pipe) = child.stdin.take() {
        if let Some(text) = stdin {
            let _ = pipe.write_all(text.as_bytes());
        }
    }
    // Drain both pipes concurrently so output above the pipe buffer cannot block glab.
    let drain = |pipe: Option<Box<dyn Read + Send>>| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(mut p) = pipe {
                let _ = p.read_to_end(&mut buf);
            }
            buf
        })
    };
    let out_reader = drain(child.stdout.take().map(|p| Box::new(p) as _));
    let err_reader = drain(child.stderr.take().map(|p| Box::new(p) as _));
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("glab timed out".into());
            }
            Ok(None) if cancel.load(Ordering::SeqCst) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("cancelled".into());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(25)),
            Err(_) => return Err("glab failed".into()),
        }
    };
    Ok(Output {
        status,
        stdout: out_reader.join().unwrap_or_default(),
        stderr: err_reader.join().unwrap_or_default(),
    })
}

pub(super) fn text_of(out: &Output) -> String {
    format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

// shortcut: relies on glab's config.yml layout, confirm in 02-SPIKE.md
fn has_stored_login(dir: &Path, host: &str) -> bool {
    let Ok(text) = std::fs::read_to_string(dir.join("config.yml")) else {
        return false;
    };
    text.contains("token:") && text.lines().any(|l| l.trim() == format!("{host}:"))
}

fn status_at(bin: &Path, host: &str, dir: &Path) -> ForgeStatus {
    let mut status = ForgeStatus::gitlab(host);
    // Fast path only: a missing config means nothing to ask glab about.
    if !has_stored_login(dir, host) && !dir.join("config.yml").exists() {
        return status;
    }
    let user = match run_glab(bin, host, dir, &["api", "user"], None, SHORT) {
        Ok(out) if out.status.success() => out,
        Ok(out) => {
            let text = redact(&text_of(&out), "");
            // No stored login for this host is not an error to show.
            if !has_stored_login(dir, host) {
                return status;
            }
            let error = classify(&text, host);
            if error.code == "generic" {
                eprintln!("gitlab status: generic failure: {}", text.lines().last().unwrap_or(""));
            }
            status.error = Some(error);
            return status;
        }
        Err(e) => {
            eprintln!("gitlab status: {e}");
            status.error = Some(Code::Generic.error(host));
            return status;
        }
    };
    let json: serde_json::Value = serde_json::from_slice(&user.stdout).unwrap_or_default();
    let Some(login) = json["username"].as_str() else {
        status.error = Some(Code::Generic.error(host));
        return status;
    };
    status.login = Some(login.to_string());
    status.logged_in = true;
    if let Ok(out) = run_glab(bin, host, dir, &["api", "personal_access_tokens/self"], None, SHORT) {
        if out.status.success() {
            let own: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_default();
            status.expires_at = own["expires_at"].as_str().map(str::to_string);
            if let Some(code) = token_problem(&own) {
                status.logged_in = false;
                status.login = None;
                status.error = Some(code.error(host));
            }
        }
    }
    status
}

fn login_at(bin: &Path, host: &str, dir: &Path, token: String) -> Result<ForgeStatus, String> {
    let args = [
        "auth",
        "login",
        "--hostname",
        host,
        "--stdin",
        "--api-protocol",
        "https",
        "--git-protocol",
        "https",
        "--insecure-storage",
    ];
    let had_login = has_stored_login(dir, host);
    let out = run_glab(bin, host, dir, &args, Some(&format!("{token}\n")), SHORT)?;
    let text = redact(&text_of(&out), &token);
    drop(token); // shortcut: no zeroize crate, the String is only dropped
    if !out.status.success() {
        // A rejected login leaves the previous working credential in place.
        if !had_login {
            let _ = logout_at(bin, host, dir);
        }
        return Err(classify(&text, host).message);
    }
    let status = status_at(bin, host, dir);
    if let Some(error) = status.error {
        let _ = logout_at(bin, host, dir);
        return Err(error.message);
    }
    if !status.logged_in {
        let _ = logout_at(bin, host, dir);
        return Err(Code::Generic.message(host));
    }
    Ok(status)
}

fn logout_at(bin: &Path, host: &str, dir: &Path) -> Result<(), String> {
    let out = run_glab(bin, host, dir, &["auth", "logout", "--hostname", host], None, SHORT)?;
    let text = redact(&text_of(&out), "");
    if out.status.success() || text.to_lowercase().contains("not logged in") {
        return Ok(());
    }
    Err(format!(
        "Could not disconnect GitLab: {}",
        text.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("")
    ))
}

// Serializes glab calls that read or write the shared config.yml.
static GLAB_LOCK: std::sync::RwLock<()> = std::sync::RwLock::new(());

fn glab_write() -> std::sync::RwLockWriteGuard<'static, ()> {
    GLAB_LOCK.write().unwrap_or_else(|e| e.into_inner())
}

pub(super) fn glab_read() -> std::sync::RwLockReadGuard<'static, ()> {
    GLAB_LOCK.read().unwrap_or_else(|e| e.into_inner())
}

pub(crate) fn login(host: &str, token: &str) -> Result<ForgeStatus, String> {
    let _guard = glab_write();
    login_at(Path::new("glab"), host, &config_dir(), token.to_string())
}

pub(crate) fn status(host: &str) -> ForgeStatus {
    let _guard = glab_read();
    status_at(Path::new("glab"), host, &config_dir())
}

pub(crate) fn logout(host: &str) -> Result<(), String> {
    let _guard = glab_write();
    logout_at(Path::new("glab"), host, &config_dir())
}

#[cfg(test)]
#[path = "auth_tests.rs"]
mod tests;
