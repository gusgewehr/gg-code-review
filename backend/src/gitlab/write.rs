//! JSON writes through `glab api --input <file>`: the body lives in a 0600 temp file, never in argv.

use super::api::TOKEN_MESSAGE;
use super::auth::{classify, glab_read, redact, run_glab_cancellable, text_of};
use serde_json::Value;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub(super) enum WriteError {
    /// 400/409/422: GitLab understood the request and refused it (bad position, duplicate).
    Rejected(String),
    NotFound(String),
    Other(String),
}

impl WriteError {
    pub(super) fn message(self) -> String {
        match self {
            Self::Rejected(m) | Self::NotFound(m) | Self::Other(m) => m,
        }
    }
}

/// The HTTP status glab reports on stderr: its `(HTTP 404)` / `HTTP 404` suffix, else a status leading a line
/// (`glab: 404 Not Found`). Stdout (the response body) is never read, and ids or paths elsewhere in a line never
/// count, so an MR iid like 400 cannot turn a 5xx into a rejection.
fn http_status(stderr: &str) -> Option<u16> {
    let code = |s: &str| {
        let digits: String = s.chars().take_while(char::is_ascii_digit).collect();
        (digits.len() == 3).then(|| digits.parse().ok()).flatten()
    };
    stderr
        .rfind("HTTP ")
        .and_then(|at| code(&stderr[at + 5..]))
        .or_else(|| {
            stderr.lines().find_map(|line| {
                let line = line.trim();
                code(
                    line.strip_prefix("glab:")
                        .or_else(|| line.strip_prefix("ERROR:"))
                        .unwrap_or(line)
                        .trim_start(),
                )
            })
        })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn api_write_at(
    bin: &Path,
    host: &str,
    dir: &Path,
    method: &str,
    endpoint: &str,
    body: Option<&Value>,
    what: &str,
    backoff_ms: u64,
    cancel: &AtomicBool,
) -> Result<Vec<u8>, WriteError> {
    // Held for the whole call so Drop removes the file after glab has read it.
    let prepared = body
        .map(|b| crate::github::prepare_review(&b.to_string()))
        .transpose()
        .map_err(|_| WriteError::Other(format!("Could not prepare {what} for GitLab.")))?;
    let path = prepared.as_ref().map(|p| p.path().to_string_lossy().into_owned());
    let args: Vec<&str> = match &path {
        Some(p) => vec![
            "api",
            "-X",
            method,
            "-H",
            "Content-Type: application/json",
            "--input",
            p,
            endpoint,
        ],
        None => vec!["api", "-X", method, endpoint],
    };
    let generic = format!("Could not write {what} to {host}. Check the connection and the token.");
    let mut attempt = 0u32;
    loop {
        if cancel.load(Ordering::SeqCst) {
            return Err(WriteError::Other("cancelled".into()));
        }
        let out = {
            let _guard = glab_read();
            run_glab_cancellable(bin, host, dir, &args, None, Duration::from_secs(120), cancel)
        }
        // A timeout or spawn failure is never retried: the write may have landed (recover by marker listing).
        .map_err(|e| WriteError::Other(if e == "cancelled" { e } else { generic.clone() }))?;
        if out.status.success() {
            return Ok(out.stdout);
        }
        let status = http_status(&String::from_utf8_lossy(&out.stderr));
        match status {
            Some(401) => return Err(WriteError::Other(TOKEN_MESSAGE.into())),
            Some(403) => {
                return Err(WriteError::Other(format!(
                "GitLab refused to write {what}: the token needs the api scope and at least Reporter access to this project."
            )))
            }
            Some(400 | 409 | 422) => return Err(WriteError::Rejected(format!("GitLab rejected {what}."))),
            Some(404) => {
                return Err(WriteError::NotFound(format!(
                    "GitLab could not find the target of {what}."
                )))
            }
            _ => {}
        }
        let retry = status == Some(429) || (method != "POST" && matches!(status, Some(500 | 502 | 503 | 504)));
        if attempt < 3 && retry {
            std::thread::sleep(Duration::from_millis(backoff_ms << attempt));
            attempt += 1;
            continue;
        }
        eprintln!(
            "gitlab write: {} for {what}",
            classify(&redact(&text_of(&out), ""), host).code
        );
        return Err(WriteError::Other(generic));
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::super::test_fake::{bodies, calls, setup};
    use super::*;

    fn run(bin: &Path, dir: &Path, method: &str, body: Option<&Value>) -> Result<Vec<u8>, WriteError> {
        api_write_at(
            bin,
            "h",
            dir,
            method,
            "projects/1/x",
            body,
            "note",
            0,
            &AtomicBool::new(false),
        )
    }

    #[test]
    fn post_sends_json_via_input_file_only() {
        let (dir, bin) = setup("w-ok", "echo '{\"id\":1}'");
        let body = serde_json::json!({"body": "secret text glpat-ABCDEFGHIJKLMNOP"});
        let out = run(&bin, &dir, "POST", Some(&body)).ok().unwrap();
        assert_eq!(String::from_utf8_lossy(&out).trim(), "{\"id\":1}");
        let log = calls(&dir);
        assert_eq!(log.len(), 1);
        assert!(log[0].starts_with("api -X POST -H Content-Type: application/json --input "));
        assert!(log[0].ends_with(" projects/1/x"));
        assert!(!log[0].contains("secret text") && !log[0].contains("glpat-"));
        assert_eq!(bodies(&dir), vec![body]);
        let tmp = log[0].split("--input ").nth(1).unwrap().split(' ').next().unwrap();
        assert!(!Path::new(tmp).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn post_500_is_not_retried() {
        let (dir, bin) = setup("w-500", "echo '500 Internal Server Error' >&2; exit 1");
        let err = run(&bin, &dir, "POST", Some(&serde_json::json!({}))).err().unwrap();
        assert!(matches!(err, WriteError::Other(_)));
        assert_eq!(calls(&dir).len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn post_retries_429_then_succeeds() {
        let body = r#"n=$(wc -l < "$(dirname "$0")/args.log")
if [ "$n" -lt 3 ]; then echo "429 Too Many Requests" >&2; exit 1; fi
echo '{}'"#;
        let (dir, bin) = setup("w-429", body);
        assert!(run(&bin, &dir, "POST", Some(&serde_json::json!({}))).is_ok());
        assert_eq!(calls(&dir).len(), 3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn put_retries_502() {
        let body = r#"n=$(wc -l < "$(dirname "$0")/args.log")
if [ "$n" -lt 2 ]; then echo "502 Bad Gateway" >&2; exit 1; fi
echo '{}'"#;
        let (dir, bin) = setup("w-502", body);
        assert!(run(&bin, &dir, "PUT", Some(&serde_json::json!({}))).is_ok());
        assert_eq!(calls(&dir).len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn statuses_are_classified_with_fixed_messages() {
        for (name, stderr, kind) in [
            ("w-400", "400 Bad Request glpat-LEAKLEAKLEAK123", 0),
            ("w-422", "422 Unprocessable", 0),
            ("w-404", "404 Not Found", 1),
            ("w-401", "401 Unauthorized", 2),
            ("w-h400", "glab: 400 Bad request - Note is invalid (HTTP 400)", 0),
            ("w-h404", "glab: HTTP 404", 1),
            ("w-h401", "glab: 401 Unauthorized (HTTP 401)", 2),
        ] {
            let (dir, bin) = setup(name, &format!("echo '{stderr}' >&2; exit 1"));
            let err = run(&bin, &dir, "POST", Some(&serde_json::json!({}))).err().unwrap();
            match (kind, &err) {
                (0, WriteError::Rejected(_)) | (1, WriteError::NotFound(_)) => {}
                (2, WriteError::Other(m)) => assert_eq!(m, TOKEN_MESSAGE),
                _ => panic!("{name}: wrong class"),
            }
            assert!(!err.message().contains("glpat-"));
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    #[test]
    fn ids_and_paths_never_count_as_statuses() {
        // A 5xx on MR !400 whose body echoes ids: neither Rejected (fallback duplicate) nor a token error.
        let script = r#"echo '{"id":401,"iid":400,"message":"404"}'
echo 'glab: 500 Internal Server Error POST projects/1/merge_requests/400/discussions (HTTP 500)' >&2; exit 1"#;
        let (dir, bin) = setup("w-ids", script);
        let err = run(&bin, &dir, "POST", Some(&serde_json::json!({}))).err().unwrap();
        assert!(matches!(&err, WriteError::Other(m) if m != TOKEN_MESSAGE));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(
            http_status("Post \"https://h/api/v4/projects/1/merge_requests/409/notes\": EOF"),
            None
        );
    }

    #[test]
    fn delete_has_no_body_args() {
        let (dir, bin) = setup("w-del", "echo '{}'");
        assert!(run(&bin, &dir, "DELETE", None).is_ok());
        assert_eq!(calls(&dir), vec!["api -X DELETE projects/1/x"]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
