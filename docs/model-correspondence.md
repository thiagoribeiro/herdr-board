# Corresponding models

A task can use a declared counterpart when its model becomes unavailable. This is separate from escalating to a
stronger model because work failed a check or a review. The selection is deterministic: no extra model call ranks
candidates or invents a substitute.

## The proposed Claude/Codex profile

```sh
herdr-board models catalog --init --profile corresponding
herdr-board models equivalents
herdr-board models equivalents pesado
```

Initialization refuses to overwrite an existing catalog. It changes no primary model, role setting, permission or
budget. The ordinary `--init` still writes the original three-entry example. The new profile contains these
**proposed operational pairings**, pinned to names checked on 2026-11-25:

| Task family | Claude Code | Codex |
|---|---|---|
| Small, repeatable work | `claude/claude-haiku-5-5`, low effort | `codex/gpt-6-luna`, medium effort |
| Everyday coding | `claude/claude-sonnet-5-5`, medium effort | `codex/gpt-6.2-sol`, medium effort |
| Complex engineering | `claude/claude-opus-5-5`, medium effort | `codex/gpt-6.2-astra`, high effort |

The everyday family also includes `codex/gpt-6.1-sol` at high effort, for existing installations. Cross-provider
alternatives are ordered first. These families express intended use, not equal measured quality, equal prices,
equal context windows or interchangeable subscription allowances. Evaluate the pairs on real acceptance criteria
before relying on them for critical work. Unlisted frontier, specialized, older and custom models are not guessed
into a family. The editable `cost` values are planning weights, not prices.

The profile follows the task categories in [Claude's model guidance](https://code.claude.com/docs/en/model-config)
and [OpenAI's model selection guide](https://developers.openai.com/api/docs/guides/model-selection). The actual
program/account model list still validates a candidate before handover. Inspection does not probe it with an LLM
prompt and does not guarantee that its service is healthy or its subscription has allowance.

## Declaring a pair

For an existing custom catalog, edit a copy outside the board home and apply it with
`herdr-board models catalog --import /path/to/models.toml`. Import validates every entry before replacing the file
and keeps its prior contents in `models.previous.toml`. Task agents and autopilot cannot initialize or import policy.

`models.toml` keeps each model, its effort and its intended uses in its own `[[entry]]`. Add:

```toml
[[entry]]
name = "everyday_claude"
agent = "claude/claude-sonnet-5-5"
effort = "medium"
family = "everyday"
equivalents = ["everyday_codex"]

[[entry]]
name = "everyday_codex"
agent = "codex/gpt-6.2-sol"
effort = "medium"
family = "everyday"
equivalents = ["everyday_claude"]
```

Both sides need explicit program/model names and the same nonempty family. Missing targets, self references,
duplicate models and cross-family edges are refused with reasons. Reciprocal pairs are allowed; per-task tried
lists prevent cycles. An agent used by several catalog entries is ambiguous: a routed task's exact entry resolves
that ambiguity; otherwise no correspondence is inferred. A program with no explicit model (`claude`, `codex`) is
not mapped to a presumed model. Pin the role or route to the desired model first.

`escalate_to` remains the separate quality chain, checked for cycles. A routed task that changes to an equivalent
keeps its quality tier, so a subsequent failed review escalates from that tier rather than counting the substitute
as a quality escalation.

## Enabling and controlling handover

```sh
herdr-board set fallback.corresponding true
# Or only for one project/task:
herdr-board set fallback.corresponding true --repo my-app
herdr-board set fallback.corresponding true --task my-app/12
```

The setting is off by default. It applies to **task workers and reviewers**, including routed workers. Explicit
`worker.fallback` and `reviewer.fallback` lists take precedence, including an explicit `none` that disables handover.
Correspondents are consulted only for recognized availability failures: exhausted allowance, lost login,
unsupported/unavailable models or infrastructure failures after the normal retry policy. They are not consulted
for failed checks, review findings, content/policy refusals, context/output limits or an ordinary stop.

With this policy enabled, a refused named model does not silently switch to the program's unspecified default model.
No eligible counterpart means the task waits with its actual error and a note naming what was unavailable. A missing
reviewer program likewise does not silently select the worker's default model. Existing manual behavior remains when
the setting is off.

A counterpart uses its catalog entry's effort; an effort explicitly set on the task wins. Program allowlists,
installed versions, role support and the task's native `ask`/`auto`/`full` access still apply. Budgets and independent
review continue to apply. The existing worktree and branch stay, and the new session receives original requirements,
the previous agent's closing words, commits, dirty-state information and open findings. This preserves work and
evidence; it does not copy another program's private session state or hidden reasoning.

`details` and the task panel's handover history identify the substitute. Corresponding handovers also record
`reasonType: availability`, `correspondingEntry` and `family`. Failed candidates do not replace the previous task's
identity; exhausted peers stop instead of looping. A new delegation begins on the primary again.

Coordinators use the separate [coordinator fallback](coordinator-fallback.md) setting and recovery protocol.
Standalone planners and tool-free helpers keep their existing lifecycle. This task policy does not start an
interactive coding agent as a summarizer.
