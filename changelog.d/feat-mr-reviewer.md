### Added

- Review one GitLab merge request on demand: `scripts/review-mr.sh <merge request URL>` runs the
  review pipeline in a single throwaway container and writes `reviews/mr-<iid>/review.md` with the
  summary and every finding, suggestion and question, ready to copy into the merge request by hand.
  Nothing is posted to GitLab; the token needs only `read_api` and `read_repository`. See
  `docs/mr-review.md`.

### Removed

- The web app, the site and the Docker Compose deployment. This branch keeps only the review
  pipeline and the on-demand merge request reviewer.
