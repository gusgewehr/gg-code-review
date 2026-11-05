//! MR notes: listing through discussions, general-note writes, status notes and marker lookups.

use super::api::{api_at, parse_pages, UNEXPECTED};
use super::auth::config_dir;
use super::sanitize::{finalize_body, strip_zwsp};
use super::write::{api_write_at, WriteError};
use crate::github::{ExistingOcrComment, IssueCommentUpsert, StatusComment, StatusCommentApi};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

pub(super) struct Ctx {
    pub bin: PathBuf,
    pub host: String,
    pub dir: PathBuf,
    pub backoff_ms: u64,
}

impl Ctx {
    pub(super) fn live() -> Self {
        Self {
            bin: PathBuf::from("glab"),
            host: crate::forge::host(),
            dir: config_dir(),
            backoff_ms: 1000,
        }
    }

    pub(super) fn get(&self, endpoint: &str, paginate: bool, what: &str) -> Result<Vec<u8>, String> {
        api_at(
            &self.bin,
            &self.host,
            &self.dir,
            endpoint,
            paginate,
            what,
            self.backoff_ms,
            &AtomicBool::new(false),
        )
    }

    pub(super) fn write(
        &self,
        method: &str,
        endpoint: &str,
        body: Option<&Value>,
        what: &str,
    ) -> Result<Vec<u8>, WriteError> {
        api_write_at(
            &self.bin,
            &self.host,
            &self.dir,
            method,
            endpoint,
            body,
            what,
            self.backoff_ms,
            &AtomicBool::new(false),
        )
    }
}

#[derive(Deserialize)]
pub(super) struct GlAuthor {
    pub id: i64,
    pub username: String,
}

#[derive(Deserialize)]
pub(super) struct GlPosition {
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub old_line: Option<i64>,
    pub new_line: Option<i64>,
}

#[derive(Deserialize)]
pub(super) struct GlNote {
    pub id: i64,
    pub body: String,
    #[serde(default)]
    pub system: bool,
    pub author: GlAuthor,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub position: Option<GlPosition>,
}

#[derive(Deserialize)]
struct GlDiscussion {
    notes: Vec<GlNote>,
}

fn notes_endpoint(full_name: &str, number: i64) -> String {
    format!("{}/notes", super::mrs::mr_endpoint(full_name, number))
}

/// Every non-system note of the MR (outdated discussions included). Fails closed.
pub(super) fn list_notes(ctx: &Ctx, full_name: &str, number: i64) -> Result<Vec<GlNote>, String> {
    let endpoint = format!(
        "{}/discussions?per_page=100",
        super::mrs::mr_endpoint(full_name, number)
    );
    let bytes = ctx.get(&endpoint, true, "merge request discussions")?;
    Ok(parse_pages::<GlDiscussion>(&bytes)?
        .into_iter()
        .flat_map(|d| d.notes)
        .filter(|n| !n.system)
        .map(|mut n| {
            n.body = strip_zwsp(&n.body);
            n
        })
        .collect())
}

/// The single place general-note bodies are built.
pub(super) fn note_json(body: &str) -> Value {
    json!({ "body": finalize_body(body) })
}

pub(super) fn post_note(ctx: &Ctx, full_name: &str, number: i64, body: &str) -> Result<i64, String> {
    let out = ctx
        .write(
            "POST",
            &notes_endpoint(full_name, number),
            Some(&note_json(body)),
            "note",
        )
        .map_err(WriteError::message)?;
    serde_json::from_slice::<Value>(&out)
        .ok()
        .and_then(|v| v["id"].as_i64())
        .ok_or_else(|| UNEXPECTED.to_string())
}

pub(super) fn put_note(ctx: &Ctx, full_name: &str, number: i64, id: i64, body: &str) -> Result<(), String> {
    let endpoint = format!("{}/{id}", notes_endpoint(full_name, number));
    ctx.write("PUT", &endpoint, Some(&note_json(body)), "note")
        .map(|_| ())
        .map_err(WriteError::message)
}

fn delete_note(ctx: &Ctx, full_name: &str, number: i64, id: i64) -> Result<(), String> {
    let endpoint = format!("{}/{id}", notes_endpoint(full_name, number));
    match ctx.write("DELETE", &endpoint, None, "note") {
        Ok(_) | Err(WriteError::NotFound(_)) => Ok(()),
        Err(e) => Err(e.message()),
    }
}

pub(crate) struct GitLabStatusComments(Ctx);

impl GitLabStatusComments {
    #[cfg(test)]
    pub(super) fn with_ctx(ctx: Ctx) -> Self {
        Self(ctx)
    }

    pub(crate) fn live() -> Self {
        Self(Ctx::live())
    }
}

impl StatusCommentApi for GitLabStatusComments {
    fn publisher_id(&mut self) -> Result<i64, String> {
        let bytes = self.0.get("user", false, "the GitLab user")?;
        serde_json::from_slice::<Value>(&bytes)
            .ok()
            .and_then(|v| v["id"].as_i64())
            .ok_or_else(|| UNEXPECTED.to_string())
    }

    fn list(&mut self, repo: &str, number: i64) -> Result<Vec<StatusComment>, String> {
        Ok(list_notes(&self.0, repo, number)?
            .into_iter()
            .map(|n| StatusComment {
                id: n.id,
                author_id: n.author.id,
                body: n.body,
            })
            .collect())
    }

    fn post(&mut self, repo: &str, number: i64, body: &str) -> Result<i64, String> {
        post_note(&self.0, repo, number, body)
    }

    fn edit(&mut self, repo: &str, number: i64, id: i64, body: &str) -> Result<bool, String> {
        put_note(&self.0, repo, number, id, body).map(|_| true)
    }

    fn delete(&mut self, repo: &str, number: i64, id: i64) -> Result<(), String> {
        delete_note(&self.0, repo, number, id)
    }
}

pub(super) fn find_review_id_by_marker_at(
    ctx: &Ctx,
    full_name: &str,
    number: i64,
    marker: &str,
) -> Result<Option<i64>, String> {
    Ok(list_notes(ctx, full_name, number)?
        .into_iter()
        .find(|n| n.body.contains(marker))
        .map(|n| n.id))
}

pub(crate) fn find_review_id_by_marker(full_name: &str, number: i64, marker: &str) -> Result<Option<i64>, String> {
    find_review_id_by_marker_at(&Ctx::live(), full_name, number, marker)
}

pub(super) fn upsert_issue_comment_by_marker_at(
    ctx: &Ctx,
    full_name: &str,
    number: i64,
    marker: &str,
    body: &str,
) -> Result<IssueCommentUpsert, String> {
    match find_review_id_by_marker_at(ctx, full_name, number, marker)? {
        Some(id) => put_note(ctx, full_name, number, id, body).map(|_| IssueCommentUpsert { id, created: false }),
        None => post_note(ctx, full_name, number, body).map(|id| IssueCommentUpsert { id, created: true }),
    }
}

pub(crate) fn upsert_issue_comment_by_marker(
    full_name: &str,
    number: i64,
    marker: &str,
    body: &str,
) -> Result<IssueCommentUpsert, String> {
    upsert_issue_comment_by_marker_at(&Ctx::live(), full_name, number, marker, body)
}

/// Empty on a listing error, like the GitHub counterpart.
pub(super) fn existing_ocr_comments(ctx: &Ctx, full_name: &str, number: i64) -> Vec<ExistingOcrComment> {
    let Ok(notes) = list_notes(ctx, full_name, number) else {
        return Vec::new();
    };
    notes
        .into_iter()
        .filter_map(|n| {
            let pos = n.position.as_ref();
            let path = pos.and_then(|p| p.new_path.clone().or_else(|| p.old_path.clone()));
            let line = pos.and_then(|p| p.new_line.or(p.old_line));
            ExistingOcrComment::from_github(&json!({
                "body": n.body,
                "user": {"login": n.author.username},
                "html_url": format!("https://{}/{full_name}/-/merge_requests/{number}#note_{}", ctx.host, n.id),
                "path": path.unwrap_or_default(),
                "line": line,
            }))
        })
        .collect()
}

pub(crate) fn dedupe_review_comments(full_name: &str, number: i64, comments: Value) -> crate::github::DedupeResult {
    crate::github::dedupe_against(&existing_ocr_comments(&Ctx::live(), full_name, number), comments)
}
