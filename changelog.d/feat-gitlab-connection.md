### Added

- Connect a self-hosted GitLab. Set `OCR_FORGE=gitlab` and `OCR_GITLAB_HOST`, optionally point
  `OCR_GITLAB_CA_FILE` at an internal CA certificate (mounted for the server only), then connect in
  Settings > Integrations with a personal access token with the `api` scope.

### Changed

- The server pins the forge and host on first start and refuses to start if they change over
  existing data. GitHub deploys pin `github` and the host already detected from `OCR_GH_HOST`,
  `GH_HOST` or the `gh` login; if no host can be detected nothing is pinned. GitHub Enterprise
  deploys keep starting without extra configuration.
