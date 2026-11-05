//! Publishes the GitHub-shaped review payload as MR discussions: inline first, summary note last.

use super::diffs::{diffs_at, find_file, position_for};
use super::mrs::live_state_at;
use super::notes::{current_user_id, list_notes, own_note_with_marker, post_note, put_note, Ctx};
use super::write::WriteError;
use crate::github::extract_ocr_fingerprint;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};

const SUMMARY_MARKER: &str = "<!-- ocr-review-post:v1";
const CHANGED: &str = "The merge request changed after the review (new push or closed); publication aborted.";
const PENDING: &str =
    "GitLab has not finished processing the latest push for this merge request; publish again in a moment.";

/// Tags every inline note of one run, so a retry after a partial publication can tell its own notes apart.
pub(crate) fn review_part_marker(run_id: &str, head_sha: &str) -> String {
    format!("<!-- ocr-review-part:v1 run={run_id} head={head_sha} -->")
}

fn mr_path(full_name: &str, number: i64) -> String {
    super::mrs::mr_endpoint(full_name, number)
}

fn check_head(ctx: &Ctx, full_name: &str, number: i64, commit_id: &str, cancel: &AtomicBool) -> Result<(), String> {
    let live = live_state_at(&ctx.bin, &ctx.host, &ctx.dir, full_name, number, cancel, ctx.backoff_ms)?;
    if live.state != "OPEN" || live.head_sha != commit_id {
        return Err(CHANGED.into());
    }
    Ok(())
}

/// GitLab builds the MR version of a push asynchronously, so wait briefly for it to reach the reviewed head.
fn latest_version(
    ctx: &Ctx,
    full_name: &str,
    number: i64,
    commit_id: &str,
    cancel: &AtomicBool,
) -> Result<Value, String> {
    let endpoint = format!("{}/versions", mr_path(full_name, number));
    for attempt in 0..4u32 {
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_millis(ctx.backoff_ms << (attempt - 1)));
        }
        if cancel.load(Ordering::SeqCst) {
            return Err("cancelled".into());
        }
        let bytes = ctx.get(&endpoint, false, "merge request versions")?;
        let versions: Value = serde_json::from_slice(&bytes).map_err(|_| super::api::UNEXPECTED.to_string())?;
        if versions[0]["head_commit_sha"].as_str() == Some(commit_id) {
            return Ok(versions[0].clone());
        }
    }
    Err(PENDING.into())
}

/// Fingerprint line first (the dedupe key), then where the finding belongs, then the original text.
fn fallback_body(body: &str, path: &str, line: i64) -> String {
    let (first, rest) = body.split_once('\n').unwrap_or((body, ""));
    if first.starts_with("<!-- ocr:fingerprint=") {
        format!("{first}\n`{path}:{line}` (could not be placed on the diff)\n\n{rest}")
    } else {
        format!("`{path}:{line}` (could not be placed on the diff)\n\n{body}")
    }
}

fn post_general(ctx: &Ctx, full_name: &str, number: i64, body: &str) -> Result<(), String> {
    post_note(ctx, full_name, number, body).map(|_| ())
}

fn post_comment(
    ctx: &Ctx,
    full_name: &str,
    number: i64,
    position: Option<Value>,
    body: &str,
    fallback: &str,
) -> Result<(), String> {
    let Some(position) = position else {
        return post_general(ctx, full_name, number, fallback);
    };
    let endpoint = format!("{}/discussions", mr_path(full_name, number));
    let payload = json!({ "body": super::sanitize::finalize_body(body), "position": position });
    match ctx.write("POST", &endpoint, Some(&payload), "discussion") {
        Ok(out) => {
            let created: Value = serde_json::from_slice(&out).unwrap_or(Value::Null);
            let note = &created["notes"][0];
            if note["type"].as_str() == Some("DiffNote") {
                return Ok(());
            }
            // Created as a plain note: edit it into the fallback instead of posting a second one.
            let id = note["id"].as_i64().ok_or_else(|| super::api::UNEXPECTED.to_string())?;
            put_note(ctx, full_name, number, id, fallback)
        }
        Err(WriteError::Rejected(_)) => post_general(ctx, full_name, number, fallback),
        Err(e) => Err(e.message()),
    }
}

pub(super) fn post_review_at(
    ctx: &Ctx,
    full_name: &str,
    number: i64,
    payload: &Value,
    cancel: &AtomicBool,
) -> Result<Option<i64>, String> {
    // payload["event"] is deliberately never read: GitLab publication only writes notes.
    let commit_id = payload["commit_id"].as_str().unwrap_or_default();
    check_head(ctx, full_name, number, commit_id, cancel)?;

    let latest = latest_version(ctx, full_name, number, commit_id, cancel)?;
    let sha = |k: &str| latest[k].as_str().unwrap_or_default().to_string();
    let (base_sha, start_sha) = (sha("base_commit_sha"), sha("start_commit_sha"));

    let summary = payload["body"].as_str().unwrap_or_default();
    let marker = summary.lines().rev().find(|l| l.starts_with(SUMMARY_MARKER));
    let part = marker
        .and_then(|m| m.split_whitespace().find_map(|t| t.strip_prefix("run=")))
        .map(|run| review_part_marker(run, commit_id));
    let notes = list_notes(ctx, full_name, number)?;
    let mut seen: HashSet<String> = notes.iter().filter_map(|n| extract_ocr_fingerprint(&n.body)).collect();
    let existing_summary = match marker {
        Some(m) => own_note_with_marker(&notes, current_user_id(ctx)?, m),
        None => None,
    };
    let files = diffs_at(&ctx.bin, &ctx.host, &ctx.dir, full_name, number, cancel)?;

    for comment in payload["comments"].as_array().into_iter().flatten() {
        if cancel.load(Ordering::SeqCst) {
            return Err("cancelled".into());
        }
        let body = comment["body"].as_str().unwrap_or_default();
        let body = part
            .as_ref()
            .map_or_else(|| body.to_string(), |part| format!("{body}\n\n{part}"));
        let body = body.as_str();
        if let Some(fp) = extract_ocr_fingerprint(body) {
            if !seen.insert(fp) {
                continue;
            }
        }
        let path = comment["path"].as_str().unwrap_or_default();
        let line = comment["line"].as_i64().unwrap_or_default();
        let side = comment["side"].as_str().unwrap_or("RIGHT");
        let position = find_file(&files, path).and_then(|f| {
            position_for(f, side, line).map(|(old, new)| {
                let mut p = json!({
                    "position_type": "text", "base_sha": base_sha, "start_sha": start_sha, "head_sha": commit_id,
                    "old_path": f.old_path, "new_path": f.new_path,
                });
                if let Some(o) = old {
                    p["old_line"] = o.into();
                }
                if let Some(n) = new {
                    p["new_line"] = n.into();
                }
                p
            })
        });
        // A push during a long loop must stop the remaining writes, not only the summary.
        check_head(ctx, full_name, number, commit_id, cancel)?;
        post_comment(ctx, full_name, number, position, body, &fallback_body(body, path, line))?;
    }

    if let Some(id) = existing_summary {
        return Ok(Some(id));
    }
    check_head(ctx, full_name, number, commit_id, cancel)?;
    let id = post_note(ctx, full_name, number, summary)?;
    Ok(Some(id))
}

pub(crate) fn post_review(
    full_name: &str,
    number: i64,
    prepared: crate::github::PreparedReview,
) -> Result<Option<i64>, String> {
    let payload: Value = std::fs::read(prepared.path())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .ok_or_else(|| "The review payload could not be read.".to_string())?;
    post_review_at(&Ctx::live(), full_name, number, &payload, &AtomicBool::new(false))
}
