# Changelog

## Unreleased

### The Coordinator

- When a task's run ends (ready to approve, done, failed, stopped at a limit), the board puts its closing note, pull
  request and review in the chat of the coordinator that added it, as a `Board:` message, and the coordinator sums it
  up for you there. A research task's result is that summary: its content, never a link or a file
  ([the Coordinator](docs/coordinator.md#when-a-task-ends-it-tells-you-in-the-chat)).
- A request that continues a task is a follow-up of it: `add … --follow-up <id>` keeps the link (`details` shows it)
  and hands the new task's agent that task's change, pull request or branch, closing note and review, to keep to that
  scope. The Coordinator asks you first when it is not sure which task, or how far
  ([continuing earlier work](docs/coordinator.md#continuing-earlier-work)).

### Research

- A research result, and any report, is the agent's closing note, brought to the Coordinator's chat. A file in the
  task's reports folder, with its private link, only when you ask for one ([reports and research](docs/research.md)).

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
