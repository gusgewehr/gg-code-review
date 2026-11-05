//! Hosting-provider seam: code outside `github/` goes through here.

use std::sync::OnceLock;

pub(crate) mod guard;

#[allow(unused_imports)]
pub(crate) use crate::github::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Forge {
    GitHub,
    GitLab,
}

static FORGE: OnceLock<Forge> = OnceLock::new();
static HOST: OnceLock<String> = OnceLock::new();

pub(crate) fn current() -> Forge {
    FORGE.get().copied().unwrap_or(Forge::GitHub)
}

/// Pinned host of this deploy; "github.com" when none was recorded.
pub(crate) fn host() -> String {
    HOST.get().cloned().unwrap_or_else(|| "github.com".to_string())
}

pub(crate) fn provider_name(forge: Forge) -> &'static str {
    match forge {
        Forge::GitHub => "github",
        Forge::GitLab => "gitlab",
    }
}

/// Pure selection. GitHub takes the host detected by `github::gh_host_detected`
/// (None => no host to pin); GitLab requires a valid `OCR_GITLAB_HOST`.
pub(crate) fn configured(
    forge: Option<&str>,
    gitlab_host: Option<&str>,
    gh_detected: Option<&str>,
) -> Result<(Forge, Option<String>), String> {
    let forge = forge.map(|v| v.trim().to_ascii_lowercase()).unwrap_or_default();
    match forge.as_str() {
        "" | "github" => Ok((
            Forge::GitHub,
            gh_detected
                .map(|h| h.trim().to_ascii_lowercase())
                .filter(|h| is_valid_gh_host(h)),
        )),
        "gitlab" => {
            let host = gitlab_host.map(|h| h.trim().to_ascii_lowercase()).unwrap_or_default();
            if !is_valid_gh_host(&host) {
                return Err(
                    "OCR_FORGE=gitlab requires OCR_GITLAB_HOST as a bare host such as gitlab.example.com (no scheme or path)."
                        .to_string(),
                );
            }
            Ok((Forge::GitLab, Some(host)))
        }
        other => Err(format!("OCR_FORGE={other:?} is not supported: use github or gitlab.")),
    }
}

pub(crate) fn init_from_env() -> Result<(Forge, Option<String>), String> {
    let forge_var = std::env::var("OCR_FORGE").ok();
    let is_gitlab = forge_var
        .as_deref()
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("gitlab"));
    // gh is only consulted for GitHub deploys.
    let detected = if is_gitlab { None } else { gh_host_detected() };
    let gitlab_host = std::env::var("OCR_GITLAB_HOST").ok();
    let (forge, host) = configured(forge_var.as_deref(), gitlab_host.as_deref(), detected.as_deref())?;
    let _ = FORGE.set(forge);
    if let Some(host) = &host {
        let _ = HOST.set(host.clone());
    }
    Ok((forge, host))
}

pub(crate) fn require_github(what: &str) -> Result<(), String> {
    if current() == Forge::GitLab {
        return Err(format!("{what} is not supported yet on GitLab."));
    }
    Ok(())
}

impl Forge {
    pub(crate) fn commit_url(self, full_name: &str, sha: &str) -> String {
        match self {
            Forge::GitHub => format!("https://github.com/{full_name}/commit/{sha}"),
            Forge::GitLab => format!("https://{}/{full_name}/-/commit/{sha}", host()),
        }
    }

    pub(crate) fn review_url(self, full_name: &str, number: i64, review_id: i64) -> String {
        match self {
            Forge::GitHub => format!("https://github.com/{full_name}/pull/{number}#pullrequestreview-{review_id}"),
            Forge::GitLab => format!("https://{}/{full_name}/-/merge_requests/{number}", host()),
        }
    }

    pub(crate) fn pr_head_ref(self, number: i64) -> String {
        match self {
            Forge::GitHub => format!("refs/pull/{number}/head"),
            Forge::GitLab => format!("refs/merge-requests/{number}/head"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_urls_and_refspec_are_unchanged() {
        assert_eq!(current(), Forge::GitHub);
        assert_eq!(
            Forge::GitHub.commit_url("o/r", "abc123"),
            "https://github.com/o/r/commit/abc123"
        );
        assert_eq!(
            Forge::GitHub.review_url("o/r", 7, 42),
            "https://github.com/o/r/pull/7#pullrequestreview-42"
        );
        assert_eq!(Forge::GitHub.pr_head_ref(7), "refs/pull/7/head");
    }

    #[test]
    fn configured_selects_forge_and_host() {
        // Detection found nothing: no host to pin.
        assert_eq!(configured(None, None, None), Ok((Forge::GitHub, None)));
        assert_eq!(
            configured(Some("github"), None, Some("GHE.Example.com")),
            Ok((Forge::GitHub, Some("ghe.example.com".to_string())))
        );
        assert_eq!(configured(None, None, Some("bad host")), Ok((Forge::GitHub, None)));
        assert_eq!(
            configured(Some("gitlab"), Some("gitlab.empresa.local:8443"), None),
            Ok((Forge::GitLab, Some("gitlab.empresa.local:8443".to_string())))
        );
        for host in [None, Some(""), Some("https://x"), Some("a b")] {
            assert!(configured(Some("gitlab"), host, None)
                .unwrap_err()
                .contains("OCR_GITLAB_HOST"));
        }
        assert!(configured(Some("bitbucket"), None, None)
            .unwrap_err()
            .contains("github or gitlab"));
    }

    #[test]
    fn gitlab_urls_and_refspec() {
        assert_eq!(
            Forge::GitLab.commit_url("g/p", "abc"),
            "https://github.com/g/p/-/commit/abc"
        );
        assert_eq!(
            Forge::GitLab.review_url("g/p", 7, 0),
            "https://github.com/g/p/-/merge_requests/7"
        );
        assert_eq!(Forge::GitLab.pr_head_ref(7), "refs/merge-requests/7/head");
    }
}
