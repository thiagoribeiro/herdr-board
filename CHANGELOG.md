# Changelog

## Unreleased

### Review

- The review stays in the board: a round's result is no longer a comment on the task's pull request. It is in the task
  panel, `board details` and the review page, and its findings go to the agent, as before. `review.prComments true` (for
  a task, a repo, everything or in the repo's own file) writes the comment again ([the gate](docs/gate.md)). Your own comments on the pull
  request are still read as your feedback.
