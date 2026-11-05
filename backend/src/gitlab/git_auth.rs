//! Git-over-HTTPS auth for the GitLab host. Token only in the git child's env.

use std::process::Command;

// shortcut: assumes glab config.yml layout hosts:/<host>:/token:, confirm in 02-SPIKE.md
pub(super) fn token_from_config_text(text: &str, host: &str) -> Option<String> {
    let indent = |l: &str| l.len() - l.trim_start().len();
    let mut lines = text.lines();
    let host_indent = lines.by_ref().find_map(|l| {
        let t = l.trim();
        [format!("{host}:"), format!("\"{host}\":"), format!("'{host}':")]
            .contains(&t.to_string())
            .then(|| indent(l))
    })?;
    for l in lines {
        if l.trim().is_empty() {
            continue;
        }
        if indent(l) <= host_indent {
            return None;
        }
        if let Some(v) = l.trim().strip_prefix("token:") {
            let v = v.trim();
            let v = v
                .strip_prefix("!!null")
                .or_else(|| v.strip_prefix("!!str"))
                .unwrap_or(v)
                .trim();
            let v = v.trim_matches(|c| c == '"' || c == '\'');
            return (!v.is_empty()).then(|| v.to_string());
        }
    }
    None
}

pub(super) fn apply_at(cmd: &mut Command, host: &str, token: &str) {
    cmd.env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", format!("http.https://{host}/.extraheader"))
        .env(
            "GIT_CONFIG_VALUE_0",
            format!(
                "AUTHORIZATION: basic {}",
                crate::github::basic_auth_for("oauth2", token)
            ),
        )
        .env("GIT_TERMINAL_PROMPT", "0");
}

/// Read per call so a Reconnect in Settings takes effect without restart.
pub(crate) fn apply_git_auth(cmd: &mut Command) {
    let host = crate::forge::host();
    let text = {
        let _guard = super::auth::glab_read();
        std::fs::read_to_string(super::auth::config_dir().join("config.yml")).unwrap_or_default()
    };
    match token_from_config_text(&text, &host) {
        Some(token) => apply_at(cmd, &host, &token),
        None => {
            cmd.env("GIT_TERMINAL_PROMPT", "0");
        }
    }
}
