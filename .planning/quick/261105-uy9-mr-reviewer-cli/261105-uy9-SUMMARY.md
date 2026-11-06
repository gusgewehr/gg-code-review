---
quick_id: 261105-uy9
status: complete
branch: feat/mr-reviewer
commits: [01386e1, de9c58a, 529e407]
---

# Quick 261105-uy9 summary: on-demand GitLab MR reviewer

## Done

- `backend/src/bench/mr.rs` + `ocr-bench mr <url>`: parses the MR URL, reads `GITLAB_TOKEN` once and
  removes it from the environment, loads the MR (`/api/v4/projects/:path/merge_requests/:iid`), fetches
  the target branch and `refs/merge-requests/:iid/head` into a root-only `/work/repo` (token only in
  the git child env), runs the unchanged ReviewBench pipeline (suggestions on, 60 min deadline) and
  renders `/output/mr-<iid>/review.md`. Unit tests: URL parsing, report rendering.
- `docker/bench-entrypoint.sh`: `mr <url>` mode, internal CA from `/run/ocr-ca/ca.crt` appended to
  the system bundle, `GITLAB_TOKEN` unset for the runner and egress proxy.
- `scripts/review-mr.sh` (builds images on first run, passes tokens by name), `docs/mr-review.md`,
  `changelog.d/feat-mr-reviewer.md`, rewritten `README.md`.
- `.gitattributes`: shell scripts check out with LF (CRLF from `core.autocrlf` broke entrypoints).
- Removed the web app, site, web image, compose deployment, pnpm hooks/tooling, bench adapters,
  compose-only GitLab doc and static compose checks.

## Verified

- `cargo fmt --check` + `cargo clippy --workspace --all-targets -D warnings`: clean.
- `cargo test --lib bench::`: 9 passed.
- `sh -n` on both scripts.

## Not verified / follow-ups

- A real run against the user's GitLab (needs their tokens and CA).
- `.github/workflows` (ci, docker-smoke, release, changelog) still reference removed frontend and
  compose files and will fail if run on this branch.
- `AGENTS.md` / `CLAUDE.md` still describe the full product.
