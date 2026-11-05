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

// Provider dispatch: these shadow the `crate::github::*` glob above, so callers stay unchanged.

pub(crate) fn list_workspaces() -> Result<Vec<crate::db::WorkspaceInput>, String> {
    match current() {
        Forge::GitHub => crate::github::list_workspaces(),
        Forge::GitLab => crate::gitlab::list_workspaces(),
    }
}

pub(crate) fn list_repos(workspace_login: &str) -> Result<Vec<crate::db::RepoInput>, String> {
    match current() {
        Forge::GitHub => crate::github::list_repos(workspace_login),
        Forge::GitLab => crate::gitlab::list_repos(workspace_login),
    }
}

pub(crate) fn list_open_prs(full_name: &str) -> Result<Vec<crate::github::GhPullRequest>, String> {
    match current() {
        Forge::GitHub => crate::github::list_open_prs(full_name),
        Forge::GitLab => crate::gitlab::list_open_prs(full_name),
    }
}

pub(crate) fn list_open_prs_for_login(
    full_name: &str,
    login: &str,
) -> Result<Vec<crate::github::GhPullRequest>, String> {
    match current() {
        Forge::GitHub => crate::github::list_open_prs_for_login(full_name, login),
        Forge::GitLab => crate::gitlab::list_open_prs_for_login(full_name, login),
    }
}

pub(crate) fn live_pr_state(full_name: &str, number: i64) -> Result<crate::github::LivePrState, String> {
    match current() {
        Forge::GitHub => crate::github::live_pr_state(full_name, number),
        Forge::GitLab => crate::gitlab::live_pr_state(full_name, number),
    }
}

pub(crate) fn live_pr_state_cancellable(
    full_name: &str,
    number: i64,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<crate::github::LivePrState, String> {
    match current() {
        Forge::GitHub => crate::github::live_pr_state_cancellable(full_name, number, cancel),
        Forge::GitLab => crate::gitlab::live_pr_state_cancellable(full_name, number, cancel),
    }
}

pub(crate) fn pr_description(full_name: &str, number: i64) -> Result<(String, String), String> {
    match current() {
        Forge::GitHub => crate::github::pr_description(full_name, number),
        Forge::GitLab => crate::gitlab::pr_description(full_name, number),
    }
}

pub(crate) fn current_login() -> Option<String> {
    match current() {
        Forge::GitHub => crate::github::current_login(),
        Forge::GitLab => crate::gitlab::current_login(),
    }
}

/// Author commands are GitHub-only.
pub(crate) fn activation_pr(full_name: &str, number: i64) -> Result<Option<crate::github::GhPullRequest>, String> {
    match current() {
        Forge::GitHub => crate::github::activation_pr(full_name, number),
        Forge::GitLab => Ok(None),
    }
}

pub(crate) fn gh_host() -> String {
    match current() {
        Forge::GitHub => crate::github::gh_host(),
        Forge::GitLab => host(),
    }
}

pub(crate) fn apply_git_https_auth(command: &mut std::process::Command) {
    match current() {
        Forge::GitHub => crate::github::apply_git_https_auth(command),
        Forge::GitLab => crate::gitlab::apply_git_auth(command),
    }
}

pub(crate) fn post_review(
    full_name: &str,
    number: i64,
    prepared: crate::github::PreparedReview,
) -> Result<Option<i64>, String> {
    match current() {
        Forge::GitHub => crate::github::post_review(full_name, number, prepared),
        Forge::GitLab => crate::gitlab::post_review(full_name, number, prepared),
    }
}

pub(crate) fn find_review_id_by_marker(full_name: &str, number: i64, marker: &str) -> Result<Option<i64>, String> {
    match current() {
        Forge::GitHub => crate::github::find_review_id_by_marker(full_name, number, marker),
        Forge::GitLab => crate::gitlab::find_review_id_by_marker(full_name, number, marker),
    }
}

pub(crate) fn diff_anchors(
    full_name: &str,
    number: i64,
) -> std::collections::HashMap<String, (std::collections::HashSet<i64>, std::collections::HashSet<i64>)> {
    match current() {
        Forge::GitHub => crate::github::diff_anchors(full_name, number),
        Forge::GitLab => crate::gitlab::diff_anchors(full_name, number),
    }
}

pub(crate) fn dedupe_review_comments(
    full_name: &str,
    number: i64,
    comments: serde_json::Value,
) -> crate::github::DedupeResult {
    match current() {
        Forge::GitHub => crate::github::dedupe_review_comments(full_name, number, comments),
        Forge::GitLab => crate::gitlab::dedupe_review_comments(full_name, number, comments),
    }
}

pub(crate) fn snapshot_review_requests(
    full_name: &str,
    number: i64,
) -> Result<crate::github::ReviewRequestSnapshot, String> {
    match current() {
        Forge::GitHub => crate::github::snapshot_review_requests(full_name, number),
        Forge::GitLab => Ok(crate::github::ReviewRequestSnapshot {
            login: String::new(),
            requested: Default::default(),
            events: Default::default(),
        }),
    }
}

pub(crate) fn restore_review_requests(
    full_name: &str,
    number: i64,
    review_id: i64,
    snapshot: &crate::github::ReviewRequestSnapshot,
) -> Result<bool, String> {
    match current() {
        Forge::GitHub => crate::github::restore_review_requests(full_name, number, review_id, snapshot),
        // GitLab notes never consume review requests, so there is nothing to restore.
        Forge::GitLab => Ok(true),
    }
}

pub(crate) fn upsert_issue_comment_by_marker(
    full_name: &str,
    number: i64,
    marker: &str,
    body: &str,
) -> Result<crate::github::IssueCommentUpsert, String> {
    match current() {
        Forge::GitHub => crate::github::upsert_issue_comment_by_marker(full_name, number, marker, body),
        Forge::GitLab => crate::gitlab::upsert_issue_comment_by_marker(full_name, number, marker, body),
    }
}

/// Name kept so call sites stay unchanged; dispatches to the active forge.
pub(crate) struct GitHubStatusComments;

impl crate::github::StatusCommentApi for GitHubStatusComments {
    fn publisher_id(&mut self) -> Result<i64, String> {
        match current() {
            Forge::GitHub => crate::github::GitHubStatusComments.publisher_id(),
            Forge::GitLab => crate::gitlab::GitLabStatusComments::live().publisher_id(),
        }
    }

    fn list(&mut self, repo: &str, number: i64) -> Result<Vec<crate::github::StatusComment>, String> {
        match current() {
            Forge::GitHub => crate::github::GitHubStatusComments.list(repo, number),
            Forge::GitLab => crate::gitlab::GitLabStatusComments::live().list(repo, number),
        }
    }

    fn post(&mut self, repo: &str, number: i64, body: &str) -> Result<i64, String> {
        match current() {
            Forge::GitHub => crate::github::GitHubStatusComments.post(repo, number, body),
            Forge::GitLab => crate::gitlab::GitLabStatusComments::live().post(repo, number, body),
        }
    }

    fn edit(&mut self, repo: &str, number: i64, id: i64, body: &str) -> Result<bool, String> {
        match current() {
            Forge::GitHub => crate::github::GitHubStatusComments.edit(repo, number, id, body),
            Forge::GitLab => crate::gitlab::GitLabStatusComments::live().edit(repo, number, id, body),
        }
    }

    fn delete(&mut self, repo: &str, number: i64, id: i64) -> Result<(), String> {
        match current() {
            Forge::GitHub => crate::github::GitHubStatusComments.delete(repo, number, id),
            Forge::GitLab => crate::gitlab::GitLabStatusComments::live().delete(repo, number, id),
        }
    }
}

pub(crate) fn require_github(what: &str) -> Result<(), String> {
    if current() == Forge::GitLab {
        return Err(format!("{what} is not supported yet on GitLab."));
    }
    Ok(())
}

fn gitlab_commit_url(host: &str, full_name: &str, sha: &str) -> String {
    format!("https://{host}/{full_name}/-/commit/{sha}")
}

fn gitlab_review_url(host: &str, full_name: &str, number: i64) -> String {
    format!("https://{host}/{full_name}/-/merge_requests/{number}")
}

impl Forge {
    pub(crate) fn commit_url(self, full_name: &str, sha: &str) -> String {
        match self {
            Forge::GitHub => format!("https://github.com/{full_name}/commit/{sha}"),
            Forge::GitLab => gitlab_commit_url(&host(), full_name, sha),
        }
    }

    pub(crate) fn review_url(self, full_name: &str, number: i64, review_id: i64) -> String {
        match self {
            Forge::GitHub => format!("https://github.com/{full_name}/pull/{number}#pullrequestreview-{review_id}"),
            Forge::GitLab => gitlab_review_url(&host(), full_name, number),
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
            gitlab_commit_url("gl.example.com", "g/p", "abc"),
            "https://gl.example.com/g/p/-/commit/abc"
        );
        assert_eq!(
            gitlab_review_url("gl.example.com", "g/p", 7),
            "https://gl.example.com/g/p/-/merge_requests/7"
        );
        assert_eq!(Forge::GitLab.pr_head_ref(7), "refs/merge-requests/7/head");
    }
}
