//! Fake-glab tests for the GitLab notes layer and the publisher (PUB-01..07).

use super::notes::*;
use super::publish::{post_review_at, review_part_marker};
use super::test_fake::{bodies, calls, setup};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

const HEAD: &str = "headsha";
const MARKER: &str = "<!-- ocr-review-post:v1 run=1 -->";
const DIFF: &str = r#"[{"old_path":"a.rs","new_path":"a.rs","diff":"@@ -1,3 +1,3 @@\n ctx\n-old\n+new\n"}]"#;

fn script(extra: &str) -> String {
    format!(
        r#"d="$(dirname "$0")"
case "$*" in
{extra}
*"-X POST"*discussions*) cat "$d/disc_resp.json";;
*"-X POST"*notes*) echo '{{"id":900}}';;
*"-X PUT"*) echo '{{}}';;
*"-X DELETE"*) echo '{{}}';;
*discussions?per_page*) cat "$d/disc.json";;
*versions*) cat "$d/versions.json";;
*diffs*) cat "$d/diffs.json";;
*branches*) echo '{{"commit":{{"id":"b"}}}}';;
*merge_requests/7) cat "$d/mr.json";;
"api user") echo '{{"id":1,"username":"bot"}}';;
esac"#
    )
}

fn put(dir: &Path, name: &str, text: &str) {
    std::fs::write(dir.join(name), text).unwrap();
}

fn mr_json(state: &str, sha: &str) -> String {
    format!(
        r#"{{"iid":7,"title":"t","author":{{"username":"u"}},"sha":"{sha}","target_branch":"main","state":"{state}"}}"#
    )
}

fn fixture(name: &str, extra: &str) -> (PathBuf, Ctx) {
    let (dir, bin) = setup(name, &script(extra));
    put(&dir, "mr.json", &mr_json("opened", HEAD));
    put(
        &dir,
        "versions.json",
        &format!(r#"[{{"base_commit_sha":"B","start_commit_sha":"S","head_commit_sha":"{HEAD}"}}]"#),
    );
    put(&dir, "diffs.json", DIFF);
    put(&dir, "disc.json", "[]");
    put(
        &dir,
        "disc_resp.json",
        r#"{"id":"d","notes":[{"id":5,"type":"DiffNote"}]}"#,
    );
    let ctx = Ctx {
        bin,
        host: "h".into(),
        dir: dir.clone(),
        backoff_ms: 0,
    };
    (dir, ctx)
}

fn fp(n: u8) -> String {
    format!("<!-- ocr:fingerprint=fnv64:{n:016x} -->")
}

fn comment(n: u8, path: &str, line: i64, side: &str) -> Value {
    json!({"path": path, "line": line, "side": side, "body": format!("{}\nOpenCodeReview\n\n**[Finding] t{n}**\n\nDetail {n}", fp(n))})
}

fn payload(comments: Vec<Value>) -> Value {
    json!({"commit_id": HEAD, "event": "COMMENT", "body": format!("Summary\n\n{MARKER}"), "comments": comments})
}

fn three() -> Value {
    payload(vec![
        comment(1, "a.rs", 2, "RIGHT"),
        comment(2, "a.rs", 2, "LEFT"),
        comment(3, "a.rs", 1, "RIGHT"),
    ])
}

fn run(ctx: &Ctx, p: &Value) -> Result<Option<i64>, String> {
    post_review_at(ctx, "g/p", 7, p, &AtomicBool::new(false))
}

fn writes(dir: &Path) -> Vec<String> {
    calls(dir).into_iter().filter(|c| c.contains("-X ")).collect()
}

fn cleanup(dir: &Path) {
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn happy_path_posts_positions_then_summary_last() {
    let (dir, ctx) = fixture("p-happy", "");
    assert_eq!(run(&ctx, &three()), Ok(Some(900)));
    let w = writes(&dir);
    assert_eq!(w.len(), 4);
    assert!(w[..3].iter().all(|c| c.ends_with("merge_requests/7/discussions")));
    assert!(w[3].ends_with("merge_requests/7/notes"));
    assert!(w
        .iter()
        .all(|c| c.contains("-H Content-Type: application/json") && c.contains("--input")));
    let b = bodies(&dir);
    for (i, (old, new)) in [(None, Some(2)), (Some(2), None), (Some(1), Some(1))]
        .into_iter()
        .enumerate()
    {
        let pos = &b[i]["position"];
        assert_eq!(pos["position_type"], "text");
        assert_eq!(
            (
                pos["base_sha"].as_str(),
                pos["start_sha"].as_str(),
                pos["head_sha"].as_str()
            ),
            (Some("B"), Some("S"), Some(HEAD))
        );
        assert_eq!(pos["old_path"], "a.rs");
        assert_eq!(pos["new_path"], "a.rs");
        assert_eq!(pos.get("old_line").and_then(Value::as_i64), old);
        assert_eq!(pos.get("new_line").and_then(Value::as_i64), new);
    }
    assert!(b[3]["body"].as_str().unwrap().contains(MARKER));
    let log = calls(&dir).join("\n");
    assert!(!log.contains("Detail") && !log.contains("glpat-") && !log.contains("approve"));
    assert!(!w.iter().any(|c| c.ends_with("/merge")));
    cleanup(&dir);
}

#[test]
fn rejected_position_falls_back_to_general_note_with_fingerprint() {
    let (dir, ctx) = fixture(
        "p-400",
        r#"*"-X POST"*discussions*) echo "400 Bad Request" >&2; exit 1;;"#,
    );
    assert_eq!(run(&ctx, &payload(vec![comment(1, "a.rs", 2, "RIGHT")])), Ok(Some(900)));
    let w = writes(&dir);
    assert_eq!(w.len(), 3);
    assert!(w[1].ends_with("merge_requests/7/notes"));
    let b = bodies(&dir);
    let body = b[1]["body"].as_str().unwrap();
    assert!(body.starts_with(&fp(1)));
    assert!(body.contains("`a.rs:2`") && body.contains("Detail 1"));
    cleanup(&dir);
}

#[test]
fn non_diffnote_is_edited_in_place() {
    let (dir, ctx) = fixture("p-plain", "");
    put(&dir, "disc_resp.json", r#"{"id":"d","notes":[{"id":77,"type":null}]}"#);
    assert_eq!(run(&ctx, &payload(vec![comment(1, "a.rs", 2, "RIGHT")])), Ok(Some(900)));
    let w = writes(&dir);
    assert_eq!(w.len(), 3);
    assert!(w[1].contains("-X PUT") && w[1].ends_with("merge_requests/7/notes/77"));
    assert!(bodies(&dir)[1]["body"].as_str().unwrap().starts_with(&fp(1)));
    cleanup(&dir);
}

#[test]
fn line_outside_the_diff_goes_straight_to_a_general_note() {
    let (dir, ctx) = fixture("p-nopos", "");
    assert_eq!(
        run(&ctx, &payload(vec![comment(1, "a.rs", 99, "RIGHT")])),
        Ok(Some(900))
    );
    assert!(!writes(&dir).iter().any(|c| c.ends_with("/discussions")));
    assert_eq!(writes(&dir).len(), 2);
    cleanup(&dir);
}

#[test]
fn existing_fingerprints_and_summary_are_not_reposted() {
    let (dir, ctx) = fixture("p-dedupe", "");
    let notes: Vec<Value> = (1..=3)
        .map(|n| json!({"id": n, "body": format!("{}\nx", fp(n as u8)), "author": {"id": 1, "username": "bot"}}))
        .chain([json!({"id": 55, "body": format!("Summary\n\n{MARKER}"), "author": {"id": 1, "username": "bot"}})])
        .collect();
    put(&dir, "disc.json", &json!([{"notes": notes}]).to_string());
    assert_eq!(run(&ctx, &three()), Ok(Some(55)));
    assert!(writes(&dir).is_empty());
    cleanup(&dir);
}

#[test]
fn failed_inline_stops_before_summary_and_rerun_resumes() {
    let extra = r#"*"-X POST"*discussions*) n=$(cat "$d/cnt" 2>/dev/null || echo 0); n=$((n+1)); echo $n > "$d/cnt"; if [ "$n" = 2 ]; then echo "500 Internal Server Error" >&2; exit 1; fi; cat "$d/disc_resp.json";;"#;
    let (dir, ctx) = fixture("p-500", extra);
    assert!(run(&ctx, &three()).is_err());
    let w = writes(&dir);
    assert_eq!(w.len(), 2);
    assert!(w.iter().all(|c| c.ends_with("/discussions")));
    put(
        &dir,
        "disc.json",
        &json!([{"notes": [{"id": 1, "body": fp(1), "author": {"id": 1, "username": "bot"}}]}]).to_string(),
    );
    assert_eq!(run(&ctx, &three()), Ok(Some(900)));
    let w = writes(&dir);
    assert_eq!(w.len(), 5);
    assert!(w[4].ends_with("/notes"));
    cleanup(&dir);
}

#[test]
fn stale_head_aborts_before_any_write() {
    for (name, state, sha, vhead) in [
        ("p-sha", "opened", "other", HEAD),
        ("p-closed", "closed", HEAD, HEAD),
        ("p-ver", "opened", HEAD, "other"),
    ] {
        let (dir, ctx) = fixture(name, "");
        put(&dir, "mr.json", &mr_json(state, sha));
        put(
            &dir,
            "versions.json",
            &format!(r#"[{{"base_commit_sha":"B","start_commit_sha":"S","head_commit_sha":"{vhead}"}}]"#),
        );
        let err = run(&ctx, &three()).unwrap_err();
        if name == "p-sha" {
            assert!(err.contains("changed after the review"));
        }
        assert!(writes(&dir).is_empty(), "{name}");
        cleanup(&dir);
    }
}

#[test]
fn head_change_before_summary_aborts_without_summary() {
    let extra = r#"*merge_requests/7) n=$(cat "$d/mrn" 2>/dev/null || echo 0); n=$((n+1)); echo $n > "$d/mrn"; if [ "$n" -ge 2 ]; then cat "$d/mr_bad.json"; else cat "$d/mr.json"; fi;;"#;
    let (dir, ctx) = fixture("p-race", extra);
    put(&dir, "mr_bad.json", &mr_json("opened", "newer"));
    assert!(run(&ctx, &payload(vec![comment(1, "a.rs", 2, "RIGHT")])).is_err());
    let w = writes(&dir);
    assert_eq!(w.len(), 1);
    assert!(w[0].ends_with("/discussions"));
    cleanup(&dir);
}

#[test]
fn suggestion_fence_is_neutralised_and_event_ignored() {
    let (dir, ctx) = fixture("p-sugg", "");
    let mut p = payload(vec![]);
    p["event"] = json!("APPROVE");
    p["body"] = json!(format!("```suggestion\nx\n```\n\n{MARKER}"));
    assert!(run(&ctx, &p).is_ok());
    let b = bodies(&dir);
    assert_eq!(b.len(), 1);
    assert!(b[0]["body"].as_str().unwrap().contains("```diff"));
    assert!(calls(&dir)
        .iter()
        .all(|c| !c.contains("approve") && !c.ends_with("/merge")));
    cleanup(&dir);
}

#[test]
fn list_notes_flattens_pages_drops_system_and_strips_zwsp() {
    let (dir, ctx) = fixture("n-list", "");
    put(
        &dir,
        "disc.json",
        &format!(
            r#"[{{"notes":[{{"id":1,"body":"​/merge","author":{{"id":1,"username":"a"}}}},{{"id":2,"body":"sys","system":true,"author":{{"id":1,"username":"a"}}}}]}}]
[{{"notes":[{{"id":3,"body":"{MARKER}","author":{{"id":1,"username":"a"}}}}]}}]"#
        ),
    );
    let notes = list_notes(&ctx, "g/p", 7).unwrap();
    assert_eq!(notes.iter().map(|n| n.id).collect::<Vec<_>>(), vec![1, 3]);
    assert_eq!(notes[0].body, "/merge");
    assert_eq!(find_review_id_by_marker_at(&ctx, "g/p", 7, MARKER), Ok(Some(3)));
    assert_eq!(find_review_id_by_marker_at(&ctx, "g/p", 7, "<!-- none -->"), Ok(None));
    cleanup(&dir);
    let (dir, ctx) = fixture("n-fail", r#"*discussions?per_page*) echo "boom" >&2; exit 1;;"#);
    assert!(find_review_id_by_marker_at(&ctx, "g/p", 7, MARKER).is_err());
    cleanup(&dir);
}

#[test]
fn status_note_is_posted_once_then_edited_in_place() {
    let extra = r#"*"-X POST"*notes*) echo '[{"notes":[{"id":50,"body":"<!-- g1 -->hi","author":{"id":1,"username":"bot"}}]}]' > "$d/disc.json"; echo '{"id":50}';;"#;
    let (dir, ctx) = fixture("n-status", extra);
    let mut api = GitLabStatusComments::with_ctx(ctx);
    let classify = |b: &str| {
        b.strip_prefix("<!-- g")
            .and_then(|r| r.split(' ').next()?.parse::<i64>().ok())
            .map(Some)
    };
    for generation in [1, 2] {
        let body = format!("<!-- g{generation} -->hi");
        let id = crate::server::replace_owned_comment(&mut api, "g/p", 7, &classify, generation, &body);
        assert_eq!(id, Ok(50));
    }
    let w = writes(&dir);
    assert_eq!(w.iter().filter(|c| c.contains("-X POST")).count(), 1);
    assert_eq!(w.iter().filter(|c| c.contains("-X PUT")).count(), 1);
    assert!(w.iter().any(|c| c.ends_with("notes/50") && c.contains("-X PUT")));
    cleanup(&dir);
}

#[test]
fn delete_of_missing_note_is_ok() {
    let (dir, ctx) = fixture("n-del", r#"*"-X DELETE"*) echo "404 Not Found" >&2; exit 1;;"#);
    let mut api = GitLabStatusComments::with_ctx(ctx);
    use crate::github::StatusCommentApi;
    assert_eq!(api.delete("g/p", 7, 9), Ok(()));
    assert_eq!(api.publisher_id(), Ok(1));
    cleanup(&dir);
}

#[test]
fn upsert_posts_then_puts_and_neutralises_quick_actions() {
    let (dir, ctx) = fixture("n-upsert", "");
    let r = upsert_issue_comment_by_marker_at(&ctx, "g/p", 7, MARKER, "/merge now").unwrap();
    assert_eq!((r.id, r.created), (900, true));
    assert_eq!(bodies(&dir)[0]["body"], "\u{200B}/merge now");
    put(
        &dir,
        "disc.json",
        &format!(r#"[{{"notes":[{{"id":8,"body":"{MARKER}","author":{{"id":1,"username":"a"}}}}]}}]"#),
    );
    let r = upsert_issue_comment_by_marker_at(&ctx, "g/p", 7, MARKER, "again").unwrap();
    assert_eq!((r.id, r.created), (8, false));
    assert!(writes(&dir).last().unwrap().ends_with("notes/8"));
    cleanup(&dir);
}

#[test]
fn existing_diffnote_suppresses_same_fingerprint() {
    let (dir, ctx) = fixture("n-dedupe", "");
    let body = format!("{}\nOpenCodeReview\n\n**[Finding] t1**\n\nDetail 1", fp(1));
    put(
        &dir,
        "disc.json",
        &json!([{"notes": [{"id": 4, "body": body, "type": "DiffNote", "author": {"id": 1, "username": "bot"},
            "position": {"old_path": "a.rs", "new_path": "a.rs", "old_line": null, "new_line": 2}}]}])
        .to_string(),
    );
    let existing = existing_ocr_comments(&ctx, "g/p", 7, None);
    assert_eq!(existing.len(), 1);
    let res = crate::github::dedupe_against(&existing, json!([comment(1, "a.rs", 2, "RIGHT")]));
    assert_eq!(res.suppressed_count, 1);
    cleanup(&dir);
}

#[test]
fn partial_publication_retry_keeps_own_findings_and_posts_the_summary() {
    let (dir, ctx) = fixture("p-own", "");
    let own = review_part_marker("1", HEAD);
    let note = |n: u8, tag: &str| {
        let body = format!("{}\nOpenCodeReview\n\n**[Finding] t{n}**\n\nDetail {n}{tag}", fp(n));
        json!({"id": n, "body": body, "author": {"id": 1, "username": "bot"}})
    };
    // This run posted finding 1 before failing; another run already posted finding 3.
    let notes = json!([{"notes": [note(1, &format!("\n\n{own}")), note(3, "")]}]);
    put(&dir, "disc.json", &notes.to_string());
    let existing = existing_ocr_comments(&ctx, "g/p", 7, Some(&own));
    let all = json!([
        comment(1, "a.rs", 2, "RIGHT"),
        comment(2, "a.rs", 2, "LEFT"),
        comment(3, "a.rs", 1, "RIGHT")
    ]);
    let res = crate::github::dedupe_against(&existing, all);
    assert_eq!(res.suppressed_count, 1);
    let post = res.post_comments.as_array().unwrap().clone();
    assert_eq!(post.len(), 2);
    assert_eq!(run(&ctx, &payload(post)), Ok(Some(900)));
    let w = writes(&dir);
    assert_eq!(w.len(), 2);
    assert!(w[0].ends_with("/discussions") && w[1].ends_with("/notes"));
    let b = bodies(&dir);
    assert!(b[0]["body"].as_str().unwrap().contains(&own));
    assert!(b[0]["body"].as_str().unwrap().contains("Detail 2"));
    cleanup(&dir);
}
