//! GitLab access through the `glab` CLI (server only; runners never get it).

mod api;
mod auth;
pub(crate) mod ca;
mod diffs;
mod git_auth;
mod mrs;
mod notes;
mod projects;
mod publish;
mod sanitize;
#[cfg(all(test, unix))]
mod test_fake;
mod write;
pub(crate) use auth::{login, logout, parse_token_body, status, ForgeStatus};
pub(crate) use diffs::diff_anchors;
pub(crate) use git_auth::apply_git_auth;
pub(crate) use mrs::{
    current_login, list_open_prs, list_open_prs_for_login, live_pr_state, live_pr_state_cancellable, pr_description,
};
pub(crate) use notes::{
    dedupe_review_comments, find_review_id_by_marker, upsert_issue_comment_by_marker, GitLabStatusComments,
};
pub(crate) use publish::post_review;
// list_all and valid_full_name are consumed by later phase 3 plans.
#[allow(unused_imports)]
pub(crate) use projects::valid_full_name;
pub(crate) use projects::{list_all, list_repos, list_workspaces};

#[cfg(all(test, unix))]
#[path = "publish_tests.rs"]
mod publish_tests;

#[cfg(test)]
#[path = "read_tests.rs"]
mod read_tests;
