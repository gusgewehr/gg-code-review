//! GitLab access through the `glab` CLI (server only; runners never get it).

mod auth;
pub(crate) mod ca;
pub(crate) use auth::{login, logout, parse_token_body, status, ForgeStatus};
