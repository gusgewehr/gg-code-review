use super::api::*;
use super::mrs::*;
use super::projects::*;
use serde_json::Value;
use std::sync::atomic::AtomicBool;

#[test]
fn parse_pages_shapes() {
    assert_eq!(parse_pages::<Value>(br#"[{"a":1}]"#).unwrap().len(), 1);
    assert_eq!(parse_pages::<Value>(br#"[{"a":1}][{"a":2}]"#).unwrap().len(), 2);
    assert_eq!(
        parse_pages::<Value>(b"[{\"a\":1}]\n[{\"a\":2},{\"a\":3}]\n")
            .unwrap()
            .len(),
        3
    );
    assert_eq!(parse_pages::<Value>(b"").unwrap().len(), 0);
    let err = parse_pages::<Value>(br#"[{"secret-project":"#).unwrap_err();
    assert_eq!(err, "Unexpected GitLab response.");
}

#[test]
fn enc_percent_encodes() {
    assert_eq!(enc("g/sub/p"), "g%2Fsub%2Fp");
    assert_eq!(enc("feat/x#1+y"), "feat%2Fx%231%2By");
    assert_eq!(enc("aZ09-._~"), "aZ09-._~");
}

const PROJECTS: &str = r#"[
 {"path":"tool","path_with_namespace":"alice/tool","visibility":"private","namespace":{"full_path":"alice","kind":"user"}},
 {"path":"app","path_with_namespace":"g/app","visibility":"public","namespace":{"full_path":"g","kind":"group"}},
 {"path":"proj","path_with_namespace":"g/sub/proj","visibility":"internal","namespace":{"full_path":"g/sub","kind":"group"}},
 {"path":"old","path_with_namespace":"g/old","archived":true,"namespace":{"full_path":"g","kind":"group"}},
 {"path":"e","path_with_namespace":"g/e","empty_repo":true,"namespace":{"full_path":"g","kind":"group"}},
 {"path":"nv","path_with_namespace":"g/nv","namespace":{"full_path":"g","kind":"group"}},
 {"path":"x","path_with_namespace":"g/../x","namespace":{"full_path":"g/..","kind":"group"}}
]"#;

#[test]
fn projects_group_by_namespace() {
    let g = parse_projects(PROJECTS.as_bytes()).unwrap();
    let logins: Vec<_> = g.iter().map(|(w, _)| (w.login.as_str(), w.kind.as_str())).collect();
    assert_eq!(logins, [("alice", "user"), ("g", "org"), ("g/sub", "org")]);
    let sub = &g[2].1[0];
    assert_eq!(
        (sub.owner.as_str(), sub.name.as_str(), sub.workspace_login.as_str()),
        ("g/sub", "proj", "g/sub")
    );
    assert!(sub.is_private);
    let g_repos = &g[1].1;
    assert_eq!(g_repos.len(), 2);
    assert!(!g_repos.iter().find(|r| r.name == "app").unwrap().is_private);
    assert!(g_repos.iter().find(|r| r.name == "nv").unwrap().is_private);
}

#[test]
fn full_name_validation() {
    assert!(valid_full_name("g/sub/p.x-1_y"));
    for bad in ["p", "g//p", "g/./p", "g/../p", "g/p q", "g/p%2F"] {
        assert!(!valid_full_name(bad), "{bad}");
    }
}

const MRS: &str = r#"[
 {"iid":7,"title":"t","author":{"username":"bob","avatar_url":"http://a"},"draft":true,"sha":"abc","target_branch":"main","source_branch":"f","web_url":"http://u","updated_at":"2026-01-01T00:00:00Z","assignees":[{"username":"al"}],"reviewers":[{"username":"Re"}],"source_project_id":2,"target_project_id":1},
 {"iid":8,"title":"w","author":{"username":"bob"},"work_in_progress":true,"sha":"def","target_branch":"main"},
 {"iid":9,"title":"n","author":{"username":"bob"},"sha":null,"target_branch":"main"}
]"#;

#[test]
fn mrs_map_to_pull_requests() {
    let prs = parse_open_mrs(MRS.as_bytes()).unwrap();
    assert_eq!(prs.len(), 2);
    let a = &prs[0];
    assert_eq!((a.number, a.head_ref_oid.as_str(), a.is_draft), (7, "abc", true));
    assert_eq!(a.base_ref_name.as_deref(), Some("main"));
    assert_eq!(a.head_ref_name.as_deref(), Some("f"));
    assert_eq!(a.changed_files, Some(0));
    assert!(a.base_ref_oid.is_none());
    assert_eq!(a.review_requests.as_ref().unwrap()[0].login.as_deref(), Some("Re"));
    assert_eq!(a.assignees.as_ref().unwrap()[0].login.as_deref(), Some("al"));
    assert!(prs[1].is_draft);
    assert_eq!(prs[1].author.avatar_url.as_deref(), Some(""));
}

#[test]
fn state_mapping() {
    assert_eq!(map_state("opened"), "OPEN");
    assert_eq!(map_state("locked"), "OPEN");
    assert_eq!(map_state("closed"), "CLOSED");
    assert_eq!(map_state("merged"), "MERGED");
}

#[test]
fn description_null_is_empty() {
    let mr: GlMr = serde_json::from_str(
        r#"{"iid":1,"title":"title","author":{"username":"a"},"target_branch":"m","description":null}"#,
    )
    .unwrap();
    assert_eq!(description_from(mr), ("title".to_string(), String::new()));
}

#[cfg(unix)]
mod fake {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    fn setup(name: &str, script_body: &str) -> (PathBuf, PathBuf) {
        let dir = std::env::temp_dir().join(format!("ocr-readglab-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let script = format!(
            "#!/bin/sh\necho \"$*\" >> \"{d}/args.log\"\n{script_body}\n",
            d = dir.display()
        );
        let bin = dir.join("glab");
        std::fs::write(&bin, script).unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        (dir, bin)
    }

    fn calls(dir: &std::path::Path) -> Vec<String> {
        std::fs::read_to_string(dir.join("args.log"))
            .unwrap()
            .lines()
            .map(str::to_string)
            .collect()
    }

    #[test]
    fn retries_429_then_succeeds() {
        // args.log has one line per call; fail while fewer than 3 lines exist.
        let body = r#"n=$(wc -l < "$(dirname "$0")/args.log")
if [ "$n" -lt 3 ]; then echo "429 Too Many Requests" >&2; exit 1; fi
echo '[1]'"#;
        let (dir, bin) = setup("429", body);
        let out = api_at(
            &bin,
            "h",
            &dir,
            "projects?x=1",
            true,
            "projects",
            0,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(String::from_utf8_lossy(&out).trim(), "[1]");
        let log = calls(&dir);
        assert_eq!(log.len(), 3);
        assert!(log[0].starts_with("api --paginate "));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn exhausts_on_502_without_leaking() {
        let (dir, bin) = setup("502", r#"echo "502 Bad Gateway glpat-LEAKLEAKLEAK123" >&2; exit 1"#);
        let err = api_at(
            &bin,
            "h",
            &dir,
            "projects",
            true,
            "projects",
            0,
            &AtomicBool::new(false),
        )
        .unwrap_err();
        assert_eq!(
            err,
            "Could not load projects from h. Check the connection and refresh repository access."
        );
        assert_eq!(calls(&dir).len(), 4);
        assert!(!calls(&dir).iter().any(|l| l.contains("glpat-")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unauthorized_is_not_retried() {
        let (dir, bin) = setup("401", r#"echo "401 Unauthorized" >&2; exit 1"#);
        let err = api_at(&bin, "h", &dir, "user", false, "user", 0, &AtomicBool::new(false)).unwrap_err();
        assert_eq!(
            err,
            "GitLab rejected the stored token. Reconnect in GitLab connection with a new personal access token."
        );
        assert_eq!(calls(&dir).len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn live_state_open_and_merged() {
        let body = r#"case "$*" in
"api projects/g%2Fsub%2Fp/merge_requests/7") echo '{"iid":7,"title":"t","author":{"username":"a"},"sha":"headsha","target_branch":"main","state":"opened"}';;
"api projects/g%2Fsub%2Fp/merge_requests/8") echo '{"iid":8,"title":"t","author":{"username":"a"},"sha":"msha","target_branch":"main","state":"merged","merged_at":"2026-01-02T03:04:05.123Z"}';;
"api projects/g%2Fsub%2Fp/repository/branches/main") echo '{"commit":{"id":"basesha"}}';;
*) exit 1;;
esac"#;
        let (dir, bin) = setup("live", body);
        let no = AtomicBool::new(false);
        let s = live_state_at(&bin, "h", &dir, "g/sub/p", 7, &no, 0).unwrap();
        assert_eq!(
            (
                s.state.as_str(),
                s.head_sha.as_str(),
                s.base_ref.as_str(),
                s.base_sha.as_str()
            ),
            ("OPEN", "headsha", "main", "basesha")
        );
        let before = calls(&dir).len();
        let m = live_state_at(&bin, "h", &dir, "g/sub/p", 8, &no, 0).unwrap();
        assert_eq!((m.state.as_str(), m.base_sha.as_str()), ("MERGED", ""));
        assert!(m.merged_at.is_some());
        assert_eq!(calls(&dir).len(), before + 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cancel_kills_a_running_call() {
        let (dir, bin) = setup("cancel", "sleep 30");
        let cancel = AtomicBool::new(false);
        let started = std::time::Instant::now();
        let err = std::thread::scope(|s| {
            s.spawn(|| {
                std::thread::sleep(std::time::Duration::from_millis(200));
                cancel.store(true, std::sync::atomic::Ordering::SeqCst);
            });
            api_at(&bin, "h", &dir, "user", false, "user", 0, &cancel).unwrap_err()
        });
        assert_eq!(err, "cancelled");
        assert!(started.elapsed() < std::time::Duration::from_secs(10));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn open_list_fills_base_tip_once_per_branch() {
        let body = r#"case "$*" in
"api --paginate projects/g%2Fp/merge_requests?state=opened&per_page=100") echo '[{"iid":1,"title":"a","author":{"username":"u"},"sha":"h1","target_branch":"main"},{"iid":2,"title":"b","author":{"username":"u"},"sha":"h2","target_branch":"main"},{"iid":3,"title":"c","author":{"username":"u"},"sha":"h3","target_branch":"rel"}]';;
"api projects/g%2Fp/repository/branches/main") echo '{"commit":{"id":"mainsha"}}';;
"api projects/g%2Fp/repository/branches/rel") echo '{"commit":{"id":"relsha"}}';;
*) exit 1;;
esac"#;
        let (dir, bin) = setup("open-tips", body);
        let prs = list_open_at(&bin, "h", &dir, "g/p", 0).unwrap();
        let bases: Vec<_> = prs.iter().map(|p| p.base_ref_oid.as_deref()).collect();
        assert_eq!(bases, [Some("mainsha"), Some("mainsha"), Some("relsha")]);
        assert_eq!(calls(&dir).len(), 3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn open_list_survives_a_missing_target_branch() {
        let list = r#""api --paginate projects/g%2Fp/merge_requests?state=opened&per_page=100") echo '[{"iid":1,"title":"a","author":{"username":"u"},"sha":"h1","target_branch":"gone"},{"iid":2,"title":"b","author":{"username":"u"},"sha":"h2","target_branch":"main"}]';;"#;
        let body = format!(
            "case \"$*\" in\n{list}\n\"api projects/g%2Fp/repository/branches/main\") echo '{{\"commit\":{{\"id\":\"mainsha\"}}}}';;\n*) echo \"404 Not Found\" >&2; exit 1;;\nesac"
        );
        let (dir, bin) = setup("open-gone", &body);
        let prs = list_open_at(&bin, "h", &dir, "g/p", 0).unwrap();
        let bases: Vec<_> = prs.iter().map(|p| (p.number, p.base_ref_oid.as_deref())).collect();
        assert_eq!(bases, [(1, None), (2, Some("mainsha"))]);
        let _ = std::fs::remove_dir_all(&dir);

        let body = format!("case \"$*\" in\n{list}\n*) echo \"401 Unauthorized\" >&2; exit 1;;\nesac");
        let (dir, bin) = setup("open-401", &body);
        let err = list_open_at(&bin, "h", &dir, "g/p", 0).unwrap_err();
        assert!(err.starts_with("GitLab rejected the stored token"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod git_auth_tests {
    use super::super::git_auth::*;
    use std::process::Command;

    #[test]
    fn config_token_lookup() {
        let t = "hosts:\n  other.example:\n    token: glpat-OTHER\n  gl.example.com:\n    token: glpat-MINE\n    api_host: gl.example.com\n";
        assert_eq!(
            token_from_config_text(t, "gl.example.com").as_deref(),
            Some("glpat-MINE")
        );
        for v in ["\"glpat-Q\"", "'glpat-Q'"] {
            let t = format!("hosts:\n  h:\n    token: {v}\n");
            assert_eq!(token_from_config_text(&t, "h").as_deref(), Some("glpat-Q"));
        }
        let t = "hosts:\n  h:\n    token: !!null glpat-T\n";
        assert_eq!(token_from_config_text(t, "h").as_deref(), Some("glpat-T"));
        assert_eq!(
            token_from_config_text("hosts:\n  h:\n    api_host: h\n  o:\n    token: x\n", "h"),
            None
        );
        assert_eq!(token_from_config_text("hosts:\n  h:\n    token: \n", "h"), None);
        assert_eq!(token_from_config_text("garbage", "h"), None);
    }

    #[test]
    fn extraheader_env_only() {
        let mut cmd = Command::new("git");
        apply_at(&mut cmd, "gl.example.com:8443", "glpat-X");
        let env: std::collections::HashMap<_, _> = cmd
            .get_envs()
            .map(|(k, v)| {
                (
                    k.to_string_lossy().to_string(),
                    v.unwrap().to_string_lossy().to_string(),
                )
            })
            .collect();
        assert_eq!(env["GIT_CONFIG_COUNT"], "1");
        assert_eq!(env["GIT_CONFIG_KEY_0"], "http.https://gl.example.com:8443/.extraheader");
        assert_eq!(
            env["GIT_CONFIG_VALUE_0"],
            format!(
                "AUTHORIZATION: basic {}",
                crate::github::basic_auth_for("oauth2", "glpat-X")
            )
        );
        assert_eq!(env["GIT_TERMINAL_PROMPT"], "0");
        assert_eq!(cmd.get_args().count(), 0);
        assert_eq!(
            crate::github::basic_auth_for("x-access-token", "t"),
            crate::github::basic_auth_for("x-access-token", "t")
        );
    }
}
