# Coordinator fallback

`coordinator.fallback` is an ordered, opt-in list for a coordinator whose provider becomes unavailable. It applies
to the top, machine and project coordinators, and can be set globally or for one coordinator:

```sh
herdr-board set coordinator.fallback codex/gpt-6.2-astra
herdr-board set coordinator.fallback codex/gpt-6.2-sol --project-coordinator my-app
```

The board handles exhausted allowance, lost login, unavailable models and terminal infrastructure errors without
asking the unavailable model to choose its replacement. Candidates must support the coordinator role and its current
access setting, be installed, and pass the program's model checks. A matching catalog entry supplies its effort;
otherwise the replacement uses its native default effort. The primary setting remains unchanged. A fresh coordinator
conversation returns to that primary. Explicit `none` disables automatic handover.

Before closing the old pane, the pump persists a transition. It retains durable memory, consumption, queued messages
and the previous session and transcript references. `coordinator history` lists the old conversation; `coordinator
status --json` also reports handovers and exhausted alternatives. The replacement receives recovery evidence and the
original request id, sender and delegation intent. It must inspect current task status, original requirements and
batch receipts before continuing unfinished work. An interrupted add, delegation or approval may already have worked:
replaying the original command blindly is unsafe. Ambiguous outcomes require the human's decision.

This is a new native conversation, with access to the old evidence, rather than a transfer of hidden model state.
The board does not itself repeat task operations. Each program/model is tried once per fallback chain, so reciprocal
lists cannot loop. With no eligible candidate it leaves the queue intact and names the actual failure. A normal
answer, permission dialog, content refusal, context limit, explicit stop or shared-budget stop does not trigger it.

After repairing a login or installation, `coordinator restart` clears the exhausted-candidate marker and retries
the current conversation. `coordinator new` starts on the primary instead. Queued messages remain queued in both.

Workers and reviewers have their own [corresponding-model policy](model-correspondence.md). Delivery summaries fall
back to the original closing message when the tool-free helper is unavailable; task results remain accessible.
