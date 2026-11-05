//! Fake `glab` script shared by the write and publish tests: logs argv and copies any `--input` body.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

pub(super) fn setup(name: &str, script_body: &str) -> (PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("ocr-gl-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let script = format!(
        "#!/bin/sh\necho \"$*\" >> \"{d}/args.log\"\nprev=; for a in \"$@\"; do [ \"$prev\" = --input ] && {{ cat \"$a\"; echo; }} >> \"$(dirname \"$0\")/bodies.log\"; prev=$a; done\n{script_body}\n",
        d = dir.display()
    );
    let bin = dir.join("glab");
    std::fs::write(&bin, script).unwrap();
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    (dir, bin)
}

pub(super) fn calls(dir: &Path) -> Vec<String> {
    std::fs::read_to_string(dir.join("args.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

pub(super) fn bodies(dir: &Path) -> Vec<serde_json::Value> {
    std::fs::read_to_string(dir.join("bodies.log"))
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}
