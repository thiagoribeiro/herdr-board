# Changelog

## Unreleased

### Status and stalls

- `herdr-board status`: one line per active task, with where it stands, for how long, whose turn it is (you, the
  agent, the board or GitHub) and the next step; `--json` for scripts, `--all` with done tasks too ([status](docs/status.md)).
- A task that stopped moving is found by fixed rules, with no model: too long working or merging (`stall.after`, 45m),
  a pull request GitHub finds in conflict with main, main that cannot be fetched (with the time of the last try). Its
  row, Herdr's sidebar and `status` say why, and `board.notify` hears `stalled` and `conflict`
  ([troubleshooting](docs/troubleshooting.md#a-task-is-stalled)).
- A conflict with main, found by the board's own merge or reported by GitHub on the pull request (which now starts that
  merge at once), goes back to its agent by itself, as before; `merge.fixConflicts` (on by default) turned off, it waits
  for you instead.
- Fixed: a pull request merged on GitHub while its task still said "working" was never seen; the task now goes to done,
  and `board.notify` hears `merged outside`.
- Fixed: an agent whose program exited in the middle of its turn (after "API Error: Connection lost mid-response") stayed
  "working" with nothing for `retry` to send. It is now a real failure, retried by itself when its last words are a
  passing provider error.

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
