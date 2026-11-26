# Changelog

## 1.4.1 (2026-11-25)

- The board opens once when herdr starts, and its key brings it back from anywhere (#9).
- `Ctrl+Q` jumps to the next task that needs you and opens the reply box on it (#10).
- Chat commands that skip the Coordinator: `!status [id …]`, `!start <id>`, `!stop <id>`, `!approve <id>` and
  `!tell <id> <text>` (#11).
- Your approval is tied to the commit that was reviewed (#12).

## 1.4.0 (2026-11-24)

### Coordinator quality and economy

- Per-message project/task context, compact standing rules and on-demand operating reference.
- Persistent tool-free delivery summaries (`summary.agent`), invalidated by changed evidence and shown directly in
  coordinator chat without another coordinator turn. Original messages remain available if summarization fails.
- Multi-task `status`, `summary`, `usage` and `batch` commands; batch request ids retain per-task receipts and prevent
  repeated side effects. Usage separates new input, cache creation/reads and output even with no token limit.
- Durable coordinator memory, context-budget hints and checkpoints that preserve model choices and prior sessions.
- Reviewers receive complete task requirements, including requirements beyond the former 3,000-character cutoff.

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

### The Coordinator

- A request that continues a task is a follow-up of it: `add … --follow-up <id>` keeps the link (`details` shows it)
  and hands the new task's agent that task's change, pull request or branch, closing note and review, to keep to that
  scope. The Coordinator asks you first when it is not sure which task, or how far
  ([continuing earlier work](docs/coordinator.md#continuing-earlier-work)).

### Models and routing

- A catalog of models, `~/.herdr-board/models.toml` (a built-in example until it exists): named entries such as
  `leve`, `padrao`, `pesado`, each checked like `worker.agent`. `allow.agents` refuses any program not on its list.
- Routing per task: `add`/`delegate --route <entry> --route.reason "…"`, `route.mode` (`none` or `coordinator`),
  `route.default` for tasks the board starts itself, and escalation along the entry's `escalate_to` after
  `route.escalateAfter` failed reviews, in the same worktree. A model you name yourself always wins; plans and reviews
  are never routed.
- `models catalog` and `models stats [--since 30d]`: tokens, cost, failed reviews and escalations per entry
  ([catalog and routing](docs/models.md#catalog-and-routing)).

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
