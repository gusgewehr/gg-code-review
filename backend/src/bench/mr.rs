//! `ocr-bench mr <url>`: reviews one GitLab merge request by its URL and writes a Markdown report
//! for a person to read and copy from. Nothing is posted to GitLab.
//!
//! It fetches the merge request into `/work/repo` and hands it to the ReviewBench path
//! (`reviewbench::run_from_env`), so the review is the product pipeline unchanged. The GitLab token
//! is read once, removed from the environment and only given to the `git` child that fetches.

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use serde_json::{json, Value};

const REPO: &str = "/work/repo";
const PR_DIR: &str = "/work/pr";
const OUT_DIR: &str = "/work/out";
const DEFAULT_DEADLINE_SECS: &str = "3600";

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct MrUrl {
    pub host: String,
    pub project: String,
    pub iid: i64,
}

/// `https://<host>/<group>/<sub>/<project>/-/merge_requests/<iid>[/diffs][?…][#…]`.
pub(crate) fn parse_url(url: &str) -> Result<MrUrl, String> {
    const SHAPE: &str = "Expected https://<host>/<group>/<project>/-/merge_requests/<iid>";
    let rest = url.trim().strip_prefix("https://").ok_or(SHAPE)?;
    let (host, path) = rest.split_once('/').ok_or(SHAPE)?;
    let (project, tail) = path.split_once("/-/merge_requests/").ok_or(SHAPE)?;
    let iid = tail
        .split(['/', '?', '#'])
        .next()
        .and_then(|iid| iid.parse::<i64>().ok())
        .filter(|iid| *iid > 0)
        .ok_or(SHAPE)?;
    let host_ok = !host.is_empty()
        && host
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b".-:".contains(&byte));
    let project_ok = project.contains('/')
        && project.split('/').all(|part| {
            !part.is_empty()
                && !part.starts_with('.')
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
        });
    if !host_ok || !project_ok {
        return Err(SHAPE.into());
    }
    Ok(MrUrl {
        host: host.to_ascii_lowercase(),
        project: project.to_string(),
        iid,
    })
}

pub fn run_from_env() -> Result<(), String> {
    let url = std::env::args()
        .nth(2)
        .ok_or("Usage: ocr-bench mr <merge request URL>")?;
    let mr = parse_url(&url)?;
    let token = std::env::var("GITLAB_TOKEN")
        .ok()
        .map(|token| token.trim().to_string())
        .filter(|token| !token.is_empty())
        .ok_or("GITLAB_TOKEN is required (a personal access token with scopes read_api and read_repository)")?;
    std::env::remove_var("GITLAB_TOKEN");

    let info = fetch_info(&mr, &token)?;
    let head = sha_at(&info, "/diff_refs/head_sha")?;
    let base = sha_at(&info, "/diff_refs/base_sha")?;
    let target = info["target_branch"]
        .as_str()
        .filter(|branch| !branch.is_empty())
        .ok_or("GitLab returned no target branch")?;
    eprintln!(
        "ocr-bench: fetching !{} of {} ({base:.8}...{head:.8})",
        mr.iid, mr.project
    );
    fetch_repo(&mr, &token, target, &base, &head)?;
    drop(token);

    std::fs::create_dir_all(PR_DIR).map_err(|error| error.to_string())?;
    let pr_json = json!({
        "title": info["title"].as_str().unwrap_or_default(),
        "body": info["description"].as_str().unwrap_or_default(),
    });
    std::fs::write(Path::new(PR_DIR).join("pr.json"), pr_json.to_string()).map_err(|error| error.to_string())?;
    // Under /work/out, which the runner keeps: it empties /output (its own) between stages.
    let out_dir = Path::new(OUT_DIR).join(format!("mr-{}", mr.iid));
    std::fs::create_dir_all(&out_dir).map_err(|error| error.to_string())?;

    // The pipeline names a repository `owner/name`; a GitLab path can be deeper, so the groups
    // collapse into the owner.
    let (groups, project_name) = mr.project.rsplit_once('/').ok_or("Invalid project path")?;
    for (key, value) in [
        ("RB_CONFIG_SUGGESTIONS", "true"),
        ("RB_CONFIG_LANGUAGE", "Brazilian Portuguese (pt-BR)"),
        ("RB_CONFIG_DEADLINE_SECS", DEFAULT_DEADLINE_SECS),
    ] {
        if std::env::var(key).unwrap_or_default().is_empty() {
            std::env::set_var(key, value);
        }
    }
    let out = out_dir.join("findings.json");
    for (key, value) in [
        ("RB_OUT", out.to_string_lossy().into_owned()),
        ("RB_NWO", format!("{}/{project_name}", groups.replace('/', "-"))),
        ("RB_PR_NUMBER", mr.iid.to_string()),
        ("RB_BASE", base.clone()),
        ("RB_HEAD", head.clone()),
        ("RB_REPO", REPO.to_string()),
        ("RB_PR_JSON", format!("{PR_DIR}/pr.json")),
    ] {
        std::env::set_var(key, value);
    }
    super::run_reviewbench()?;

    let report: Value = serde_json::from_slice(
        &std::fs::read(out_dir.join("ocr-findings.json")).map_err(|_| "The review wrote no report")?,
    )
    .map_err(|_| "The review report is not JSON")?;
    let markdown = render(&url, &info, &report);
    let path = out_dir.join("review.md");
    std::fs::write(&path, &markdown).map_err(|error| format!("Could not write {}: {error}", path.display()))?;
    println!("{markdown}");
    eprintln!("ocr-bench: report written to {}", path.display());
    Ok(())
}

fn fetch_info(mr: &MrUrl, token: &str) -> Result<Value, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|error| error.to_string())?;
    let endpoint = format!(
        "https://{}/api/v4/projects/{}/merge_requests/{}",
        mr.host,
        crate::gitlab::enc(&mr.project),
        mr.iid
    );
    // Error text never quotes the response or the request: the token rides in its headers.
    let response = client
        .get(endpoint)
        .header("PRIVATE-TOKEN", token)
        .send()
        .map_err(|_| {
            format!(
                "Could not reach {}. Check the network and that the internal CA is mounted at /run/ocr-ca/ca.crt.",
                mr.host
            )
        })?;
    match response.status().as_u16() {
        200 => response
            .json()
            .map_err(|_| "GitLab returned an unreadable merge request".into()),
        401 => Err("GitLab rejected GITLAB_TOKEN. Create a new token with scopes read_api and read_repository.".into()),
        403 | 404 => Err(format!(
            "Merge request !{} of {} was not found, or the token cannot read it.",
            mr.iid, mr.project
        )),
        status => Err(format!("GitLab answered HTTP {status} for the merge request.")),
    }
}

fn sha_at(info: &Value, pointer: &str) -> Result<String, String> {
    info.pointer(pointer)
        .and_then(Value::as_str)
        .filter(|sha| sha.len() == 40 && sha.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| format!("GitLab returned no {pointer} commit"))
}

/// Fetches the target branch and the merge request head into a fresh `/work/repo`, root-only so
/// the model reads only the frozen snapshot, and checks both review commits arrived.
fn fetch_repo(mr: &MrUrl, token: &str, target: &str, base: &str, head: &str) -> Result<(), String> {
    use std::os::unix::fs::DirBuilderExt;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(REPO)
        .map_err(|error| error.to_string())?;
    let git = |args: &[&str], auth: bool| -> Result<(), String> {
        let mut command = Command::new("git");
        command.arg("-C").arg(REPO).args(args);
        if auth {
            crate::gitlab::apply_git_auth_with(&mut command, &mr.host, token);
        }
        let status = command
            .status()
            .map_err(|error| format!("Could not run git: {error}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("git {} failed", args.first().copied().unwrap_or_default()))
        }
    };
    git(&["init", "-q"], false)?;
    let remote = format!("https://{}/{}.git", mr.host, mr.project);
    git(
        &[
            "fetch",
            "--quiet",
            "--no-tags",
            &remote,
            &format!("+refs/heads/{target}:refs/remotes/origin/{target}"),
            &format!("+refs/merge-requests/{}/head:refs/remotes/origin/mr", mr.iid),
        ],
        true,
    )
    .map_err(|_| {
        format!(
            "Could not fetch {} from {}. Check the token has scope read_repository and can read the project.",
            mr.project, mr.host
        )
    })?;
    for sha in [base, head] {
        if git(&["cat-file", "-e", &format!("{sha}^{{commit}}")], false).is_err() {
            // The target branch moved past a rewritten base; ask for the commit itself.
            git(&["fetch", "--quiet", "--no-tags", &remote, sha], true)
                .map_err(|_| format!("Commit {sha:.8} of the merge request is not fetchable"))?;
        }
    }
    git(&["checkout", "-q", "--detach", head], false)
}

/// The report a reviewer reads and copies from: the summary, then each point with its location.
pub(crate) fn render(url: &str, info: &Value, report: &Value) -> String {
    let text = |value: &Value| value.as_str().unwrap_or_default().trim().to_string();
    let mut out = format!(
        "# Revisão do !{}: {}\n\n{}\n\n",
        info["iid"].as_i64().unwrap_or_default(),
        text(&info["title"]),
        info["web_url"].as_str().unwrap_or(url)
    );
    let findings = report["findings"].as_array().cloned().unwrap_or_default();
    let questions = report["questions"].as_array().cloned().unwrap_or_default();
    out.push_str(&format!(
        "Status: **{}** · {} · {}s · {} achados, {} perguntas\n\n",
        text(&report["status"]),
        text(&report["engine"]["label"]),
        report["durationMs"].as_i64().unwrap_or_default() / 1000,
        findings.len(),
        questions.len()
    ));
    if let Some(error) = report["error"].as_str() {
        out.push_str(&format!("> Erro da revisão: {error}\n\n"));
    }
    let summary = text(&report["summary"]);
    let (summary, _) = crate::review::split_suggestions(&summary);
    if !summary.trim().is_empty() {
        out.push_str(&format!("## Resumo\n\n{}\n\n", summary.trim()));
    }
    for (heading, points) in [("Achados", &findings), ("Perguntas", &questions)] {
        if points.is_empty() {
            continue;
        }
        out.push_str(&format!("## {heading}\n\n"));
        for (index, point) in points.iter().enumerate() {
            let (start, end) = (
                point["startLine"].as_i64().unwrap_or(1),
                point["endLine"].as_i64().unwrap_or(1),
            );
            let lines = if start == end {
                start.to_string()
            } else {
                format!("{start}-{end}")
            };
            out.push_str(&format!(
                "### {}. `{}:{lines}` · {}\n\n",
                index + 1,
                text(&point["path"]),
                kind_label(point["kind"].as_str().unwrap_or_default())
            ));
            let title = text(&point["title"]);
            if title.is_empty() {
                // A comment the export could not split: show it as it would have been posted.
                out.push_str(&format!("{}\n\n", text(&point["body"])));
                continue;
            }
            out.push_str(&format!("**{title}**\n\n"));
            let detail = text(&point["detail"]);
            if !detail.is_empty() {
                out.push_str(&format!("{detail}\n\n"));
            }
            let action = text(&point["action"]);
            if !action.is_empty() {
                let label = if point["kind"] == "question" {
                    "A confirmar"
                } else {
                    "Correção sugerida"
                };
                out.push_str(&format!("**{label}:** {action}\n\n"));
            }
        }
    }
    out
}

fn kind_label(kind: &str) -> &str {
    match kind {
        "finding" => "achado",
        "suggestion" => "sugestão",
        "question" => "pergunta",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_merge_request_urls() {
        assert_eq!(
            parse_url("https://gitlab.corp.example:8443/group/sub/app/-/merge_requests/42/diffs?view=x#note").unwrap(),
            MrUrl {
                host: "gitlab.corp.example:8443".into(),
                project: "group/sub/app".into(),
                iid: 42
            }
        );
        for bad in [
            "http://gitlab.example/g/p/-/merge_requests/1",
            "https://gitlab.example/p/-/merge_requests/1",
            "https://gitlab.example/g/p/-/merge_requests/0",
            "https://gitlab.example/g/p/-/issues/1",
            "https://gitlab.example/g/../p/-/merge_requests/1",
            "https://gitlab.example/g/p q/-/merge_requests/1",
            "https://user@gitlab.example/g/p/-/merge_requests/1",
        ] {
            assert!(parse_url(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn renders_points_for_copying() {
        let info = json!({"iid": 7, "title": "Add cache", "web_url": "https://g/x/y/-/merge_requests/7"});
        let report = json!({
            "status": "ok",
            "engine": {"label": "claude/opus/high"},
            "durationMs": 61000,
            "error": null,
            "summary": "Two problems.\n\n### 💡 Suggestions\n\n- `a.rs:1` **x**. y Suggested change: z",
            "findings": [
                {"kind": "finding", "path": "src/a.rs", "startLine": 3, "endLine": 5,
                 "title": "Cache never expires", "detail": "Entries grow forever.", "action": "Add a TTL.", "body": ""},
                {"kind": "suggestion", "path": "src/b.rs", "startLine": 9, "endLine": 9,
                 "title": "", "detail": "", "action": "", "body": "Raw comment"}
            ],
            "questions": []
        });
        let markdown = render("u", &info, &report);
        assert!(markdown.starts_with("# Revisão do !7: Add cache\n\nhttps://g/x/y/-/merge_requests/7"));
        assert!(markdown.contains("**ok** · claude/opus/high · 61s · 2 achados, 0 perguntas"));
        assert!(markdown.contains("## Resumo\n\nTwo problems.\n\n"));
        assert!(!markdown.contains("Suggested change"));
        assert!(markdown.contains(
            "### 1. `src/a.rs:3-5` · achado\n\n**Cache never expires**\n\nEntries grow forever.\n\n**Correção sugerida:** Add a TTL."
        ));
        assert!(markdown.contains("### 2. `src/b.rs:9` · sugestão\n\nRaw comment"));
        assert!(!markdown.contains("## Perguntas"));
    }
}
