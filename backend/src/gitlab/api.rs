//! `glab api` runner: bounded retry, tolerant pagination, fixed error strings (no payload or stderr text).

use super::auth::{classify, config_dir, glab_read, redact, run_glab_cancellable, text_of};
use serde::de::DeserializeOwned;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

const TOKEN_MESSAGE: &str =
    "GitLab rejected the stored token. Reconnect in GitLab connection with a new personal access token.";
pub(super) const UNEXPECTED: &str = "Unexpected GitLab response.";

/// Percent-encodes everything outside the RFC 3986 unreserved set (project paths, branch names).
pub(super) fn enc(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for b in value.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// `glab api --paginate` may print concatenated arrays; accept any number of them.
pub(super) fn parse_pages<T: DeserializeOwned>(bytes: &[u8]) -> Result<Vec<T>, String> {
    let mut items = Vec::new();
    for page in serde_json::Deserializer::from_slice(bytes).into_iter::<Vec<T>>() {
        // serde errors can quote payload values (private project names), so never embed them.
        items.extend(page.map_err(|_| UNEXPECTED.to_string())?);
    }
    Ok(items)
}

fn retryable(stderr: &str) -> bool {
    stderr
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|t| matches!(t, "429" | "500" | "502" | "503" | "504"))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn api_at(
    bin: &Path,
    host: &str,
    dir: &Path,
    endpoint: &str,
    paginate: bool,
    what: &str,
    backoff_ms: u64,
    cancel: &AtomicBool,
) -> Result<Vec<u8>, String> {
    let args: Vec<&str> = if paginate {
        vec!["api", "--paginate", endpoint]
    } else {
        vec!["api", endpoint]
    };
    let generic = format!("Could not load {what} from {host}. Check the connection and refresh repository access.");
    let mut attempt = 0u32;
    loop {
        if cancel.load(Ordering::SeqCst) {
            return Err("cancelled".into());
        }
        // Read lock per attempt so a login/logout never waits out the retry backoff.
        let out = {
            let _guard = glab_read();
            run_glab_cancellable(bin, host, dir, &args, None, Duration::from_secs(120), cancel)
        }
        .map_err(|e| if e == "cancelled" { e } else { generic.clone() })?;
        if out.status.success() {
            return Ok(out.stdout);
        }
        let text = redact(&text_of(&out), "");
        let class = classify(&text, host);
        if matches!(class.code, "unauthorized" | "expired") {
            return Err(TOKEN_MESSAGE.into());
        }
        if attempt < 3 && retryable(&String::from_utf8_lossy(&out.stderr)) {
            std::thread::sleep(Duration::from_millis(backoff_ms << attempt));
            attempt += 1;
            continue;
        }
        eprintln!("gitlab api: {} for {what}", class.code);
        return Err(generic);
    }
}

pub(super) fn api(endpoint: &str, paginate: bool, what: &str) -> Result<Vec<u8>, String> {
    api_at(
        Path::new("glab"),
        &crate::forge::host(),
        &config_dir(),
        endpoint,
        paginate,
        what,
        1000,
        &AtomicBool::new(false),
    )
}
