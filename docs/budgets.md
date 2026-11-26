# Shared token budgets

`limit.tokens` caps one task. Shared budgets reserve allowance across coordinators, workers, reviewers and the
board's measured summary/Watcher calls on **one machine and one board home**. They do not coordinate spending
across machines, even when the main Coordinator can see their tasks.

Both caps are `none` until the human sets them. These commands are examples; installing this code sets no cap:

```sh
herdr-board set budget.tokens 5m
herdr-board set budget.projectTokens 2m --repo my-app
herdr-board set budget.period 24h
herdr-board set budget.reserve 100k
herdr-board set budget.reviewPercent 20
herdr-board budget --json
```

The machine cap and each project's cap both apply. The human's project entry alone holds `budget.projectTokens`;
the other four settings belong to the machine. Task settings and committed `.herdr-board.json` files cannot raise,
disable or otherwise supply them. Worker/reviewer commands and task-origin coordinator turns cannot change budgets
or reconcile consumption. Autopilot cannot do so either. A coordinator responding to a human remains subject to
its normal requirement for explicit human authorization.

## Reservations and observed spending

Before launching or delivering a prompt, the board atomically reserves up to `budget.reserve` tokens. Another
process cannot promise the same allowance to another owner. Each task and coordinator has a stable owner; retries,
reviews and coordinator checkpoints reuse its account. Individual attempts have their own reservations. Choosing
an attempt does not bill the adopted conversation twice: transcript identities and shared Claude reply ids are
deduplicated across owners.

`used` means observed consumption in the current window. `reserved` means allowance promised to unfinished or
unreconciled work. Reservations persist outside the repository, survive board/process restarts, and shrink as
confirmed consumption arrives. The task panel shows observed use and reservations separately; coordinator context
receives a compact snapshot. Neither snapshot is authorization to change the budget.

With `budget.reviewPercent 20`, non-reviewer work cannot commit the final 20% of a capped allowance; reviewers can
use that margin. This is a conservative floor against total observed use and reservations, including earlier reviews.
Workers cannot spend it by retrying or replying. An exhausted queued task stays queued. Running work is interrupted
when the board observes exhaustion or cannot establish consumption within its startup grace period; its work is kept,
automatic merging is held, and the task explains the budget stop. Retry once allowance and evidence are available.
An explicit manual reconciliation remains authoritative when the old transcript stays unavailable.

The window starts with the ledger and lasts `budget.period` (24 hours by default), rather than resetting at local
midnight. Changes to its duration take effect at its next boundary. A backward clock grants no new allowance.
Unfinished reservations and cumulative baselines survive rollover. Consumption is charged in the window where it
is observed; enrolling an existing conversation establishes its baseline instead of retrospectively billing its
earlier history. The ledger retains the last 30 completed windows' aggregates.

## Missing evidence and recovery

Supported task/coordinator transcripts include native Claude/Codex children and cache reads/writes. A program the
board cannot measure is refused under an enabled cap, with no silent model substitution. Budgeted one-shot summary
and Watcher calls currently require Claude's JSON usage envelope; another helper is deferred while that cap applies.
Deferring a summary does not block task completion or discard its original result.

Missing or unreadable usage is unknown, never zero. The owner retains its reservation, and new work in its capped
scope waits. `herdr-board budget` lists these owners. Final transcript evidence resolves the reservation automatically
when available. If it cannot, inspect the program/provider evidence and reconcile the **owner's cumulative count**:

```sh
herdr-board budget settle task:my-app/12 --used 140k
```

A live owner cannot normally be reconciled. If a helper's callback or task record was lost, first verify that its
model call has ended. The human can certify that end explicitly:

```sh
herdr-board budget settle helper:summary:my-app/12:OWNER-UUID --used 12k --ended
```

Reconciliation records that confirmation, cannot refund observed tokens, and does not start a model. Inspection and
reconciliation skip model-scheduling housekeeping. A corrupt ledger or malformed budget setting is preserved and
blocks new budgeted work; it is never silently replaced with a fresh allowance.

## Enforcement boundary

Programs report usage after calls. Polling and interrupting them cannot impose a hard provider-side spending limit:
calls already in flight, including native parallel children, can overshoot. The ledger records that overshoot and
refuses additional board-scheduled work. Cache tokens are counted separately in usage reports but included in this
token cap; this is not a monetary cost estimate.

Project caps cover that project's tasks, project coordinator and its measured helpers. The top/machine
coordinators serve several projects and belong to the machine allowance; their mixed conversation is not allocated
to individual projects. With budgets enabled, Watcher calls group tasks by project so their usage has one owner.

These controls apply to model work the board starts and can identify. They do not account for arbitrary model calls
an agent launches through shell scripts, external tools or another board home. Native `ask`/`auto` modes remain
available through each role's access setting. Strong process isolation and a broker that mediates every model call
are separate work; neither prompt instructions nor this ledger supplies them.
