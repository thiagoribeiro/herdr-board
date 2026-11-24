# Changelog

## Unreleased

### Herdr's sidebar

- Task agents stand in a tree under the coordinator that added them (the Coordinator, the machine's or a project's):
  `├─ BRD-2 …`, `└─ BRD-3 …`, and a task that waits for another (`--after`) one level under that one. The reviewer's
  entry is in its task's space, right under the task's agent. Tasks you added yourself come last, outside every tree.
  The tree is a prefix before the task's number in its space's name, and the order of the spaces: Herdr 0.9.5 nests
  nothing itself. It is drawn while Herdr sorts its agents list by space (`agent_panel_sort = "spaces"`, Herdr's
  default); in its `priority` sort the spaces keep their plain names and order, since a tree would stand in pieces
  there ([herdr's sidebar](docs/sidebar.md#the-tree-tasks-under-their-coordinator)).

### Review

- The review stays in the board: a round's result is no longer a comment on the task's pull request. It is in the task
  panel, `board details` and the review page, and its findings go to the agent, as before. `review.prComments true` (for
  a task, a repo, everything or in the repo's own file) writes the comment again ([the gate](docs/gate.md)). Your own comments on the pull
  request are still read as your feedback.
