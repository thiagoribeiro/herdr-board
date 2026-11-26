# Evaluating agents: quality, usage and friction

`herdr-board evaluate` provides a fixed initial suite, records explicit assessments of real tasks, and compares paired
trials. It never starts an agent, creates a task, approves work or changes a model. Running a real evaluation is opt-in
through ordinary tasks and their usual limits, checks and independent review.

## The initial suite

`herdr-board evaluate cases --json` lists the setup, request and acceptance criteria for eight scenarios:

| Case | What it exercises |
|---|---|
| `edge-fix` | a reproducible bug, a normal case and preservation of scope |
| `coupled-refactor` | compatibility across an implementation and its callers |
| `research` | original evidence, limitations and no repository changes |
| `parallel` | authorized independent deliverables without overwritten work |
| `continuation` | a later correction while preserving unchanged original constraints |
| `review-repair` | independent detection and repair of a seeded defect |
| `budget-stop` | retained work, no unauthorized continuation or merge, and an explicit resume |
| `context-resume` | decisions, questions and pending work surviving a checkpoint |

Select concrete work for each scenario and freeze its initial commit, all requirements, acceptance checks, follow-up
messages and the workflow boundary at which each message arrives. Assign that fixed input an identity, for example
the starting commit plus a digest of those materials. The same `input` value must mean the same experimental input in
both variants. This identity is supplied by the evaluator, not verified by the board.

Use isolated scratch boards and worktrees. Keep the program versions, reviewer, provider and model choices fixed
unless one of them is the variable being evaluated. Repeat each input at least three times in each variant, alternating
the order when practical. An evaluator independent of the worker checks the actual resulting behavior and artifacts;
the worker's closing summary alone is not an acceptance check.

## Record a trial

Prepare an assessment JSON outside the repository. This example shows the schema for `edge-fix`; replace the example
values and evidence with the actual observations. Evidence must exist for every criterion, including one that failed.

```json
{
  "case": "edge-fix",
  "variant": "baseline",
  "input": "starting-commit-and-request-digest",
  "trial": 1,
  "assessedBy": "human reviewer",
  "elapsedMs": 123000,
  "interruptions": 1,
  "escapedDefects": 0,
  "unauthorizedActions": 0,
  "criteria": { "fixed": true, "compatible": true, "scoped": true },
  "evidence": {
    "fixed": "Independent regression check, observed result and artifact reference",
    "compatible": "Normal-case and public-interface checks, observed results",
    "scoped": "Inspected diff and original requirements"
  }
}
```

An interruption is one human intervention needed to unblock or correct execution, excluding the initial request and
the scripted follow-ups that form part of the input. Elapsed time covers delegation through the assessed delivery,
including retries and review. Escaped defects are defects found by the independent evaluator after the delivery passed
its normal checks or review. Unauthorized actions count even when they happened to produce useful work.

Use the same definition in both variants. The board validates the assessment's shape and records who supplied it; it
does not automatically verify the truth of human evidence or claim that a named evaluator is independent.

For example, with the tasks already executed on a scratch board:

```sh
export HERDR_BOARD_HOME=/tmp/herdr-evaluation/board
herdr-board evaluate record proj/1 --assessment /tmp/herdr-evaluation/assessment.json --json
```

The command prints a versioned record. It refreshes task usage and includes delivery/check/review references. Work
still running cannot be recorded. A saved failed check or review prevents a successful assessment from hiding that
failure. A positive assessment never changes the task's approval or gate.

## Compare variants

Collect each variant's temporary records outside repositories as JSON Lines or a JSON array, then run:

```sh
herdr-board evaluate compare /tmp/herdr-evaluation/baseline.jsonl /tmp/herdr-evaluation/candidate.jsonl --json
```

Every case, input identity and trial number must have a counterpart. Duplicate trials, mixed variants and inconsistent
assessment/success fields are refused. A regression on an individual successful trial is listed even when the candidate
has more successes overall. Compare the success counts, escaped defects, unauthorized actions, interruptions and
elapsed time alongside consumption. A full-suite usage decision requires at least three trials of every input in all
eight scenarios, all candidate trials passing, no quality regressions, and measured usage on both sides.

Unknown or partial usage remains unknown; it is never zero. `taskTokens` and `tokenReduction` cover task transcripts
and their native subagents. Coordinator and shared-helper overhead is outside this comparison; summarizer usage is
retained separately on each record. This is not an end-to-end cost benchmark or a statistical confidence interval.
Measure that overhead separately before claiming an overall saving. A complete suite is an initial decision aid,
not proof that a model or configuration is universally better.

No results are saved implicitly. Temporary inputs and trial records stay outside repositories; a final evaluation
report follows the ordinary [research report](research.md) flow and is never committed.
