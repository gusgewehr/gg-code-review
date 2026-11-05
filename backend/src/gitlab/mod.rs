//! GitLab access through the `glab` CLI (server only; runners never get it).

mod api;
mod auth;
pub(crate) mod ca;
mod git_auth;
mod mrs;
mod projects;
pub(crate) use auth::{login, logout, parse_token_body, status, ForgeStatus};
pub(crate) use git_auth::apply_git_auth;
pub(crate) use mrs::{
    current_login, list_open_prs, list_open_prs_for_login, live_pr_state, live_pr_state_cancellable, pr_description,
};
// list_all and valid_full_name are consumed by later phase 3 plans.
#[allow(unused_imports)]
pub(crate) use projects::valid_full_name;
pub(crate) use projects::{list_all, list_repos, list_workspaces};

#[cfg(test)]
#[path = "read_tests.rs"]
mod read_tests;
