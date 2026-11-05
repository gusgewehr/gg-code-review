//! Line map of a merge request built from GitLab's own `/diffs`, never from local git.

use super::api::{api_at, enc, parse_pages};
use super::auth::config_dir;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::AtomicBool;

#[derive(Deserialize)]
pub(super) struct GlDiff {
    old_path: String,
    new_path: String,
    // Absent or empty for collapsed / too_large entries.
    #[serde(default)]
    diff: String,
}

/// `lines` holds (old, new) numbers: added = (None, new), removed = (old, None), context = both.
pub(super) struct FileDiff {
    pub old_path: String,
    pub new_path: String,
    pub lines: Vec<(Option<i64>, Option<i64>)>,
}

type Position = (Option<i64>, Option<i64>);

pub(super) fn parse_diffs(bytes: &[u8]) -> Result<Vec<FileDiff>, String> {
    Ok(parse_pages::<GlDiff>(bytes)?
        .into_iter()
        .map(|d| {
            let (mut old, mut new) = (0i64, 0i64);
            let mut lines = Vec::new();
            for line in d.diff.lines() {
                if line.starts_with("@@") {
                    if let Some((o, n)) = crate::github::parse_hunk_starts(line) {
                        (old, new) = (o, n);
                    }
                } else if line.starts_with('\\') {
                    continue;
                } else if line.starts_with('-') {
                    lines.push((Some(old), None));
                    old += 1;
                } else if line.starts_with('+') {
                    lines.push((None, Some(new)));
                    new += 1;
                } else {
                    lines.push((Some(old), Some(new)));
                    old += 1;
                    new += 1;
                }
            }
            FileDiff {
                old_path: d.old_path,
                new_path: d.new_path,
                lines,
            }
        })
        .collect())
}

/// RIGHT matches the new number, LEFT the old one; a context line answers with both.
pub(super) fn position_for(file: &FileDiff, side: &str, line: i64) -> Option<Position> {
    file.lines
        .iter()
        .find(|(o, n)| {
            if side == "LEFT" {
                *o == Some(line)
            } else {
                *n == Some(line)
            }
        })
        .copied()
}

pub(super) fn find_file<'a>(files: &'a [FileDiff], path: &str) -> Option<&'a FileDiff> {
    files
        .iter()
        .find(|f| f.new_path == path)
        .or_else(|| files.iter().find(|f| f.old_path == path))
}

/// new_path -> (right lines of added+context, left lines of removed+context), same shape as the GitHub map.
pub(super) fn anchors_from(files: &[FileDiff]) -> HashMap<String, (HashSet<i64>, HashSet<i64>)> {
    files
        .iter()
        .map(|f| {
            let right = f.lines.iter().filter_map(|(_, n)| *n).collect();
            let left = f.lines.iter().filter_map(|(o, _)| *o).collect();
            (f.new_path.clone(), (right, left))
        })
        .collect()
}

pub(super) fn diffs_at(
    bin: &Path,
    host: &str,
    dir: &Path,
    full_name: &str,
    number: i64,
    cancel: &AtomicBool,
) -> Result<Vec<FileDiff>, String> {
    let endpoint = format!("projects/{}/merge_requests/{number}/diffs?per_page=100", enc(full_name));
    parse_diffs(&api_at(
        bin,
        host,
        dir,
        &endpoint,
        true,
        "merge request diffs",
        1000,
        cancel,
    )?)
}

/// Empty map on any error: callers skip anchor checks, like the GitHub counterpart.
pub(crate) fn diff_anchors(full_name: &str, number: i64) -> HashMap<String, (HashSet<i64>, HashSet<i64>)> {
    diffs_at(
        Path::new("glab"),
        &crate::forge::host(),
        &config_dir(),
        full_name,
        number,
        &AtomicBool::new(false),
    )
    .map(|f| anchors_from(&f))
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIX: &str = r#"[
     {"old_path":"a.rs","new_path":"a.rs","diff":"@@ -10,3 +10,4 @@\n a\n-b\n+c\n+d\n e\n\\ No newline at end of file\n"},
     {"old_path":"old.rs","new_path":"new.rs","renamed_file":true,"diff":"@@ -1 +1 @@\n x\n"},
     {"old_path":"gone.rs","new_path":"gone.rs","deleted_file":true,"diff":"@@ -1,2 +0,0 @@\n-p\n-q\n"},
     {"old_path":"big.rs","new_path":"big.rs","too_large":true,"diff":""}
    ]"#;

    fn files() -> Vec<FileDiff> {
        parse_diffs(FIX.as_bytes()).unwrap()
    }

    #[test]
    fn hunk_maps_context_removed_added() {
        assert_eq!(
            files()[0].lines,
            vec![
                (Some(10), Some(10)),
                (Some(11), None),
                (None, Some(11)),
                (None, Some(12)),
                (Some(12), Some(13))
            ]
        );
    }

    #[test]
    fn position_per_side() {
        let f = files();
        assert_eq!(position_for(&f[0], "RIGHT", 11), Some((None, Some(11))));
        assert_eq!(position_for(&f[0], "LEFT", 11), Some((Some(11), None)));
        assert_eq!(position_for(&f[0], "RIGHT", 13), Some((Some(12), Some(13))));
        assert_eq!(position_for(&f[0], "LEFT", 12), Some((Some(12), Some(13))));
        assert_eq!(position_for(&f[0], "RIGHT", 99), None);
    }

    #[test]
    fn rename_delete_and_collapsed() {
        let f = files();
        assert_eq!((f[1].old_path.as_str(), f[1].new_path.as_str()), ("old.rs", "new.rs"));
        assert_eq!(find_file(&f, "old.rs").unwrap().new_path, "new.rs");
        assert_eq!(position_for(&f[2], "LEFT", 2), Some((Some(2), None)));
        assert!(f[3].lines.is_empty());
    }

    #[test]
    fn anchors_shape() {
        let a = anchors_from(&files());
        let (right, left) = &a["a.rs"];
        assert_eq!(right, &HashSet::from([10, 11, 12, 13]));
        assert_eq!(left, &HashSet::from([10, 11, 12]));
    }

    #[cfg(unix)]
    #[test]
    fn diffs_at_reads_through_glab() {
        use super::super::test_fake::{calls, setup};
        let (dir, bin) = setup(
            "diffs",
            r#"echo '[{"old_path":"a","new_path":"a","diff":"@@ -1 +1 @@"}]'"#,
        );
        let f = diffs_at(&bin, "h", &dir, "g/p", 3, &AtomicBool::new(false)).unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(
            calls(&dir)[0],
            "api --paginate projects/g%2Fp/merge_requests/3/diffs?per_page=100"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
