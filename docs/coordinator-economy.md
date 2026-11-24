# Coordinator quality and economy

The board handles routine state, dependencies and notifications. The coordinator interprets requests and makes
decisions. Task agents implement and investigate; reviewers independently check changes. Existing worker, reviewer
and coordinator models and effort are unchanged by these features.

## Context travels with the request

Every queued coordinator message captures its original repo, root, selected task, sender and delegation intent. It
also captures compact snapshots of the selected task and tasks explicitly named by id or short key. A request id
and this context arrive in a `herdr-board-context` block, hidden from the human's chat. Switching selection cannot
change which task an earlier message meant. The dated snapshots are background, not authorization or a live lock:
commands still check the current task. `coordinator where` remains available for old messages without context.

The coordinator's standing rules are shorter. `coordinator help TOPIC` loads only that topic (plan, automations,
attempts, fallback, limits, research, chat, settings, models, merge, archive, dependencies); omit the topic for the
full reference. The core rules retain human authorization, settings and limit boundaries, research/chat modes,
independent review and the obligation to inspect original evidence when a summary is insufficient.

## Persistent delivery summaries

`summary.agent` selects a tool-free, ephemeral summarizer (built in: `claude/haiku`; `none` disables generation).
Claude and pi have verified tool-free modes. Other programs are refused for this role. No tools, coding session or
permission to change the board is given to the helper. It summarizes delivery evidence as data, not instructions.

The keeper/housekeeping schedules a dedicated summary runner after a task delivers work: in approval, done or an
ended review/failure. The runner waits for its helper, so a short-lived originating command cannot lose its result.
The input is the original task requirements, closing summary, report reference/content, check evidence and review. Full
execution logs are not loaded. Inputs over 64 KB are left intact and marked unavailable instead of truncated.
An output has result, changes, reported checks/review, limitations and open questions. The board attaches its own evidence
references; the helper cannot manufacture artifact links or a reviewed commit. Invalid output is rejected.

The summary is saved on the task as `digest`. Its key covers evidence (including report content) and summary schema version. Changed
requirements, delivery, check, report, commit or review invalidate it. Progress, polling timestamps and merge state
do not. At most one helper call runs on the machine. A claim expires after a crashed process, failed calls back off
five minutes and stop after three attempts for the same evidence and program. Late output cannot replace changed
evidence. Originals are always retained; failure to summarize never approves, merges or blocks task completion.

The result is shown as an attributed task result in its coordinator's chat without queuing a coordinator turn.
While a summary is unavailable, the original closing message is displayed instead. Structured deliveries can be
reused directly without a helper call. Board-recorded checks are displayed separately from reported evidence.
`summary ID --json` returns it with evidence references and current status separately. Current workflow status is
read live from the board; the summary is historical delivery evidence, not proof that a task is still ready to merge.

## One query or operation for several tasks

`status [ID ...] [--all] [--repo NAME] --json` returns task id, title, stage, saved state, observation time,
responsible party, next action, blockers and cached delivery. Without ids it excludes done/archived tasks unless
`--all` is given. It uses no model. `summary` and `usage` accept several ids too.

`batch delegate|approve|stop|retry|review|fix|archive|restore ID ... --request-id REQUEST --json` runs the normal
guarded commands with per-task outcomes and resulting snapshots. All task ids are validated before any action.
Each item retains normal authorization, review, check, archive and limit guards. Acceptance is not completion.

A request id identifies the operation, ids and flags. Reusing it returns the persisted receipt without repeating
an action; using it for different work is refused. Each action is marked in flight before it runs. After a crash,
an uncertain action is not automatically repeated: inspect current state and decide whether to issue a new request.
Successful and failed outcomes are both recorded. A replay is labeled and refreshes current task snapshots.
Receipts are under the board's data folder, outside repositories.

## Continuity and bounded context

Messages to a selected task continue directly in its own session; they need not pass through a coordinator. A clear
instruction can be forwarded with `tell` without re-investigating the whole task. Related new work references the
previous task, decisions and evidence. Keep coupled changes together; parallel work is appropriate for independent
deliverables, with attention to overlapping files and interfaces even in isolated worktrees.

`coordinator memory JSON` stores an object of string arrays: `decisions`, `constraints`, `questions`, at most 12 KB.
It is included with later messages. Do not store secrets, raw logs or mutable workflow status. Original task
requirements and authorizations remain authoritative. The board never uses a summary as permission to act.

At a natural boundary, after recording all unresolved context, `coordinator checkpoint` starts a fresh conversation
on the same running program/model/effort and preserves memory, queued requests and original history. It must be the
last command of the turn, as it replaces that conversation. Memory must be refreshed after the latest delivered
request. `coordinator history` retains the prior session references. `coordinator.contextBudget` (160k by default)
adds a checkpoint hint when observed context exceeds it; `none` disables that hint. There is no reset after every message and no automatic
lossy replacement of an unresolved conversation. The program's own normal compaction remains available.

## Measurement and quality

`usage [ID ...] --json` counts tokens independently of `limit.tokens`. Supported transcript readers separate new
input, cache creation, cache reads, output and request counts. `coordinator usage` gives totals and the last observed
request's delta. Counters resume at the last complete transcript line; repeated Claude message parts count once.
Older counters can have a partial detailed breakdown until a new conversation, while their original total is kept.
Summary helper usage is recorded when the program returns it. Monetary cost is unknown unless separately supplied;
cache reads must not be presented as costing the same as fresh input. Concurrent inputs can share a model turn, so
per-request measurements are observed deltas rather than exact provider billing allocations.

Validate clear actions, multi-project selection, ambiguous references, repeated requests, delayed events/results,
restarts, failed review and unchanged cache hits. A reduction is a gain only if original requirements, authorization,
independent review and current evidence remain correct. No status polling loop should be created by the coordinator;
the board already schedules retries, dependencies, checks and notifications.
