### Added

- Connect a self-hosted GitLab. Set `OCR_FORGE=gitlab` and `OCR_GITLAB_HOST`, optionally point
  `OCR_GITLAB_CA_FILE` at an internal CA certificate (mounted for the server only), then connect in
  Settings > Integrations with a personal access token with the `api` scope.

- GitLab deploys list groups, subgroups and projects (Developer access or higher) in Settings >
  Repository access, discover open merge requests of the selected projects by polling (draft merge
  requests follow the per-repository draft-skip setting, closed or merged ones leave the queue, a new
  push replaces the queued review) and prepare reviews from `refs/merge-requests/<iid>/head` with a
  local diff, including merge requests from forks. Publishing results to GitLab is not available yet.

### Changed

- The server pins the forge and host on first start and refuses to start if they change over
  existing data. GitHub deploys pin `github` and the host already detected from `OCR_GH_HOST`,
  `GH_HOST` or the `gh` login; if no host can be detected nothing is pinned. GitHub Enterprise
  deploys keep starting without extra configuration.
