//! Merge requests mapped onto the GitHub pull-request types the rest of the server uses.

use super::api::{api, api_at, enc, parse_pages, TOKEN_MESSAGE, UNEXPECTED};
use super::auth::config_dir;
use crate::github::{GhPrAuthor, GhPrPrincipal, GhPullRequest, LivePrState};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Deserialize)]
struct GlUser {
    username: String,
    avatar_url: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct GlMr {
    iid: i64,
    title: String,
    author: GlUser,
    draft: Option<bool>,
    work_in_progress: Option<bool>,
    sha: Option<String>,
    target_branch: String,
    source_branch: Option<String>,
    web_url: Option<String>,
    updated_at: Option<String>,
    #[serde(default)]
    assignees: Vec<GlUser>,
    #[serde(default)]
    reviewers: Vec<GlUser>,
    state: Option<String>,
    merged_at: Option<String>,
    description: Option<String>,
}

fn principals(users: Vec<GlUser>) -> Vec<GhPrPrincipal> {
    users
        .into_iter()
        .map(|u| GhPrPrincipal {
            login: Some(u.username),
            name: None,
            slug: None,
        })
        .collect()
}

pub(super) fn to_pull_request(mr: GlMr) -> Option<GhPullRequest> {
    let sha = mr.sha.filter(|s| !s.is_empty())?;
    Some(GhPullRequest {
        number: mr.iid,
        title: mr.title,
        author: GhPrAuthor {
            login: mr.author.username,
            // Some so the GitHub avatar fallback in polling_pr.rs is never used on GitLab.
            avatar_url: Some(mr.author.avatar_url.unwrap_or_default()),
        },
        is_draft: mr.draft.or(mr.work_in_progress).unwrap_or(false),
        head_ref_oid: sha,
        base_ref_oid: None,
        base_ref_name: Some(mr.target_branch),
        head_ref_name: mr.source_branch,
        url: mr.web_url,
        updated_at: mr.updated_at,
        // shortcut: MR list has no file count; the review-time limit (db/review_runs.rs) uses local counts
        changed_files: Some(0),
        assignees: Some(principals(mr.assignees)),
        review_requests: Some(principals(mr.reviewers)),
    })
}

pub(super) fn parse_open_mrs(bytes: &[u8]) -> Result<Vec<GhPullRequest>, String> {
    Ok(parse_pages::<GlMr>(bytes)?
        .into_iter()
        .filter_map(to_pull_request)
        .collect())
}

pub(super) fn map_state(state: &str) -> String {
    match state {
        "opened" | "locked" => "OPEN".into(),
        "closed" => "CLOSED".into(),
        "merged" => "MERGED".into(),
        other => other.to_uppercase(),
    }
}

fn mr_endpoint(full_name: &str, number: i64) -> String {
    format!("projects/{}/merge_requests/{number}", enc(full_name))
}

fn get_mr(
    bin: &Path,
    host: &str,
    dir: &Path,
    endpoint: &str,
    backoff_ms: u64,
    cancel: &AtomicBool,
) -> Result<GlMr, String> {
    let bytes = api_at(bin, host, dir, endpoint, false, "merge requests", backoff_ms, cancel)?;
    serde_json::from_slice(&bytes).map_err(|_| UNEXPECTED.to_string())
}

fn branch_tip_at(
    bin: &Path,
    host: &str,
    dir: &Path,
    full_name: &str,
    branch: &str,
    backoff_ms: u64,
    cancel: &AtomicBool,
) -> Result<String, String> {
    let endpoint = format!("projects/{}/repository/branches/{}", enc(full_name), enc(branch));
    let bytes = api_at(bin, host, dir, &endpoint, false, "branches", backoff_ms, cancel)?;
    let json: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| UNEXPECTED.to_string())?;
    json["commit"]["id"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| UNEXPECTED.to_string())
}

pub(super) fn live_state_at(
    bin: &Path,
    host: &str,
    dir: &Path,
    full_name: &str,
    number: i64,
    cancel: &AtomicBool,
    backoff_ms: u64,
) -> Result<LivePrState, String> {
    let cancelled = || cancel.load(Ordering::SeqCst);
    if cancelled() {
        return Err("cancelled".into());
    }
    let mr = get_mr(bin, host, dir, &mr_endpoint(full_name, number), backoff_ms, cancel)?;
    let state = map_state(mr.state.as_deref().unwrap_or_default());
    let head_sha = mr.sha.filter(|s| !s.is_empty()).ok_or_else(|| UNEXPECTED.to_string())?;
    let base_sha = if state == "OPEN" {
        if cancelled() {
            return Err("cancelled".into());
        }
        branch_tip_at(bin, host, dir, full_name, &mr.target_branch, backoff_ms, cancel)?
    } else {
        String::new()
    };
    Ok(LivePrState {
        state,
        head_sha,
        base_ref: mr.target_branch,
        base_sha,
        merged_at: mr.merged_at.as_deref().and_then(crate::github::iso8601_to_epoch_ms),
    })
}

pub(super) fn list_open_at(
    bin: &Path,
    host: &str,
    dir: &Path,
    full_name: &str,
    backoff_ms: u64,
) -> Result<Vec<GhPullRequest>, String> {
    let endpoint = format!("projects/{}/merge_requests?state=opened&per_page=100", enc(full_name));
    let no = AtomicBool::new(false);
    let mut prs = parse_open_mrs(&api_at(
        bin,
        host,
        dir,
        &endpoint,
        true,
        "merge requests",
        backoff_ms,
        &no,
    )?)?;
    // The MR list has no base SHA; the live target tip lets polling detect base moves and retargets.
    let mut tips: HashMap<String, Option<String>> = HashMap::new();
    for pr in &mut prs {
        let branch = pr.base_ref_name.clone().unwrap_or_default();
        if !tips.contains_key(&branch) {
            let tip = match branch_tip_at(bin, host, dir, full_name, &branch, backoff_ms, &no) {
                Ok(tip) => Some(tip),
                Err(e) if e == TOKEN_MESSAGE => return Err(e),
                // A deleted target must not stop the project's polling; the upsert keeps the stored base.
                Err(_) => None,
            };
            tips.insert(branch.clone(), tip);
        }
        pr.base_ref_oid = tips.get(&branch).cloned().flatten();
    }
    Ok(prs)
}

pub(crate) fn list_open_prs(full_name: &str) -> Result<Vec<GhPullRequest>, String> {
    list_open_at(Path::new("glab"), &crate::forge::host(), &config_dir(), full_name, 1000)
}

pub(crate) fn list_open_prs_for_login(full_name: &str, login: &str) -> Result<Vec<GhPullRequest>, String> {
    let has = |l: &Option<Vec<GhPrPrincipal>>| {
        l.iter()
            .flatten()
            .any(|p| p.login.as_deref().is_some_and(|x| x.eq_ignore_ascii_case(login)))
    };
    Ok(list_open_prs(full_name)?
        .into_iter()
        .filter(|pr| has(&pr.assignees) || has(&pr.review_requests))
        .collect())
}

pub(crate) fn live_pr_state(full_name: &str, number: i64) -> Result<LivePrState, String> {
    live_pr_state_cancellable(full_name, number, &AtomicBool::new(false))
}

pub(crate) fn live_pr_state_cancellable(
    full_name: &str,
    number: i64,
    cancel: &AtomicBool,
) -> Result<LivePrState, String> {
    live_state_at(
        Path::new("glab"),
        &crate::forge::host(),
        &config_dir(),
        full_name,
        number,
        cancel,
        1000,
    )
}

pub(super) fn description_from(mr: GlMr) -> (String, String) {
    (mr.title, mr.description.unwrap_or_default())
}

pub(crate) fn pr_description(full_name: &str, number: i64) -> Result<(String, String), String> {
    get_mr(
        Path::new("glab"),
        &crate::forge::host(),
        &config_dir(),
        &mr_endpoint(full_name, number),
        1000,
        &AtomicBool::new(false),
    )
    .map(description_from)
}

pub(crate) fn current_login() -> Option<String> {
    let bytes = api("user", false, "user").ok()?;
    let json: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    json["username"].as_str().map(str::to_string)
}
