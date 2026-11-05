use super::*;

const TOKEN: &str = "glpat-TESTTOKEN0000000000";

#[test]
fn redact_masks_given_and_lookalike_tokens() {
    assert_eq!(
        redact("x glpat-AbC_123-xyz9876543 y", "glpat-AbC_123-xyz9876543"),
        "x [redacted] y"
    );
    assert_eq!(
        redact("a glpat-OtherToken12345 b glpat-short", ""),
        "a [redacted] b glpat-short"
    );
    assert!(!redact(&format!("boom {TOKEN}"), "").contains("glpat-"));
}

#[test]
fn token_body_never_echoes() {
    let err = parse_token_body(format!("\"{TOKEN}\"").as_bytes()).unwrap_err();
    assert!(!err.contains("glpat-"));
    assert_eq!(
        parse_token_body(br#"{"token":"  glpat-abcdefghij1234 \n"}"#).unwrap(),
        "glpat-abcdefghij1234"
    );
    assert!(parse_token_body(br#"{"token":""}"#).is_err());
    assert!(parse_token_body(br#"{"token":"a b"}"#).is_err());
    let long = format!(r#"{{"token":"{}"}}"#, "a".repeat(513));
    assert!(parse_token_body(long.as_bytes()).is_err());
}

#[test]
fn classify_table() {
    let net = classify("Get x: x509: certificate signed by unknown authority", "gl.corp");
    assert_eq!(net.code, "unreachable_or_tls");
    assert!(net.message.contains("gl.corp") && net.message.contains("/run/ocr-ca/ca.crt"));
    assert_eq!(classify("401 token expired", "h").code, "expired");
    assert_eq!(classify("401 Unauthorized", "h").code, "unauthorized");
    assert_eq!(classify("weird", "h").code, "generic");
}

#[test]
fn token_problem_cases() {
    use serde_json::json;
    assert_eq!(token_problem(&json!({"revoked": true})), Some(Code::Expired));
    assert_eq!(token_problem(&json!({"active": false})), Some(Code::Expired));
    assert_eq!(token_problem(&json!({"expires_at": "2000-01-01"})), Some(Code::Expired));
    assert_eq!(
        token_problem(&json!({"scopes": ["read_api"]})),
        Some(Code::MissingScope)
    );
    assert_eq!(token_problem(&json!({"scopes": ["api"], "expires_at": null})), None);
    assert_eq!(token_problem(&json!({"granular_scopes": [{}]})), None);
}

#[test]
fn glab_env_is_scrubbed() {
    let mut cmd = Command::new("glab");
    cmd.env("GITLAB_TOKEN", "x");
    apply_glab_env(&mut cmd, "h", Path::new("/c"));
    let envs: std::collections::HashMap<_, _> = cmd.get_envs().collect();
    for k in ["GITLAB_TOKEN", "GITLAB_ACCESS_TOKEN", "OAUTH_TOKEN", "GLAB_TOKEN"] {
        assert_eq!(envs[std::ffi::OsStr::new(k)], None, "{k}");
    }
    assert_eq!(
        envs[std::ffi::OsStr::new("GITLAB_HOST")],
        Some(std::ffi::OsStr::new("h"))
    );
    assert_eq!(envs[std::ffi::OsStr::new("NO_PROMPT")], Some(std::ffi::OsStr::new("1")));
}

#[cfg(unix)]
mod fake {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn setup(name: &str, user: &str, own: &str) -> (PathBuf, PathBuf) {
        let dir = std::env::temp_dir().join(format!("ocr-fakeglab-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let script = format!(
            r#"#!/bin/sh
echo "$*" >> "{d}/args.log"
case "$*" in
auth\ login*) cat > "{d}/stdin.log"; env > "{d}/env.log"; printf 'h:\n  token: x\n' > "$GLAB_CONFIG_DIR/config.yml";;
auth\ logout*) ;;
api\ user) {user};;
api\ personal_access_tokens/self) {own};;
esac
"#,
            d = dir.display()
        );
        let bin = dir.join("glab");
        std::fs::write(&bin, script).unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        (dir, bin)
    }

    #[test]
    fn login_ok_keeps_token_on_stdin_only() {
        let (dir, bin) = setup(
            "ok",
            r#"echo '{"username":"alice"}'"#,
            r#"echo '{"scopes":["api"],"expires_at":null,"active":true,"revoked":false}'"#,
        );
        let s = login_at(&bin, "h", &dir, TOKEN.into()).unwrap();
        assert!(s.logged_in && s.login.as_deref() == Some("alice") && s.expires_at.is_none());
        let args = std::fs::read_to_string(dir.join("args.log")).unwrap();
        assert!(args
            .contains("auth login --hostname h --stdin --api-protocol https --git-protocol https --insecure-storage"));
        assert!(!args.contains("glpat-"));
        assert_eq!(
            std::fs::read_to_string(dir.join("stdin.log")).unwrap(),
            format!("{TOKEN}\n")
        );
        let env = std::fs::read_to_string(dir.join("env.log")).unwrap();
        assert!(env.contains("GITLAB_HOST=h") && !env.contains("glpat-"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn login_401_is_rolled_back_without_leaking() {
        let (dir, bin) = setup("401", &format!("echo '{TOKEN} 401 Unauthorized' >&2; exit 1"), "exit 1");
        let err = login_at(&bin, "h", &dir, TOKEN.into()).unwrap_err();
        assert_eq!(err, Code::Unauthorized.message("h"));
        assert!(std::fs::read_to_string(dir.join("args.log"))
            .unwrap()
            .contains("auth logout --hostname h"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn login_without_api_scope_is_rolled_back() {
        let (dir, bin) = setup(
            "scope",
            r#"echo '{"username":"a"}'"#,
            r#"echo '{"scopes":["read_api"]}'"#,
        );
        let err = login_at(&bin, "h", &dir, TOKEN.into()).unwrap_err();
        assert_eq!(err, Code::MissingScope.message("h"));
        assert!(std::fs::read_to_string(dir.join("args.log"))
            .unwrap()
            .contains("auth logout --hostname h"));
        assert!(logout_at(&bin, "h", &dir).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
