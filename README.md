# OpenCodeReview — GitLab MR reviewer

[![License](https://img.shields.io/badge/license-PolyForm_Small_Business_1.0.0-blue.svg)](LICENSE)

This branch keeps only the review brain of [OpenCodeReview](https://github.com/BridgLabs/opencodereview)
and a minimal GitLab connection, as a tool to review a colleague's merge request on demand:

```sh
scripts/review-mr.sh https://gitlab.example.com/group/project/-/merge_requests/42
```

It fetches the merge request with your read-only token, reviews it with
[Claude Code](https://docs.anthropic.com/en/docs/claude-code) or [Codex](https://github.com/openai/codex)
in a sandboxed runner, and writes `reviews/mr-42/review.md`: a summary, then every finding,
suggestion and question with its `path:line`, explanation and suggested fix. You read it and copy
what is worth it into the merge request yourself. **Nothing is posted to GitLab.**

Setup, tokens, internal CA and options: [docs/mr-review.md](docs/mr-review.md).

## What the review does

- **Evidence-checked findings.** Every finding must quote the exact code it is about and anchor to a
  changed line; reports that fail validation are never reported as clean.
- **Chunked, multi-stage review** of large merge requests, with a lens pass for what a first pass
  usually misses and lower-priority suggestions (tests, naming, duplication).
- **Your conventions.** `AGENTS.md`, `CLAUDE.md`, `CONTRIBUTING.md`, `REVIEW.md` and similar files in
  the reviewed repository steer the review.
- **Isolated execution.** The AI CLI only sees a frozen snapshot of the change, and its network is
  limited to the AI provider. The GitLab token never reaches it.

## Layout

- `backend/src/review`, `review_archive`, `providers`, `sandbox`: the review pipeline.
- `backend/src/bench`: the headless driver (`ocr-bench mr`, `ocr-bench reviewbench`).
- `docker/Dockerfile` + `docker/Dockerfile.bench`: the image the script builds and runs.

The rest of the Rust crate (server, polling, publication) is still compiled because the pipeline
depends on its database and forge layers; it is not run.

## License

[PolyForm Small Business 1.0.0](LICENSE). See [NOTICE](NOTICE).
