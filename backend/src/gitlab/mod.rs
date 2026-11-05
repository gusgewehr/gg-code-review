//! GitLab access through the `glab` CLI (server only; runners never get it).

mod api;
mod auth;
pub(crate) mod ca;
#[allow(dead_code)] // consumed by publish/notes (04-03), allow removed in 04-04
mod diffs;
mod git_auth;
mod mrs;
#[allow(dead_code)] // consumed by forge in 04-04
mod notes;
mod projects;
#[allow(dead_code)] // consumed by forge in 04-04
mod publish;
#[allow(dead_code)] // consumed by publish/notes (04-03), allow removed in 04-04
mod sanitize;
#[cfg(all(test, unix))]
mod test_fake;
#[allow(dead_code)] // consumed by publish/notes (04-03), allow removed in 04-04
mod write;
pub(crate) use auth::{login, logout, parse_token_body, status, ForgeStatus};
pub(crate) use git_auth::apply_git_auth;
pub(crate) use mrs::{
    current_login, list_open_prs, list_open_prs_for_login, live_pr_state, live_pr_state_cancellable, pr_description,
};
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
