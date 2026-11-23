# Models and effort

**Pick a model by writing it after the program, with a slash: `codex/gpt-6.1-sol`.** Leave it out and the program uses
its own default. Effort (how hard the agent thinks) is a setting of its own. A wrong name or level is refused when you
set it, with the ones that work.

```
herdr-board models            every program, a few names each, and its effort levels
herdr-board models codex      every name codex takes, written as a setting writes them, and its effort levels
```

Every agent the board starts has a **program**, and if you want, a **model** and an **effort**. You choose them per role:
the working agent (`worker.`), the reviewer (`reviewer.`), the Coordinator (`coordinator.`), the plan's writer (`plan.`),
the Watcher (`watcher.`), and the lists that name agents (`worker.fallback`, `reviewer.fallback`, `attempts.by`).

## Examples

```
herdr-board set worker.agent codex/gpt-6.1-sol          you, for every task
herdr-board set worker.effort high
herdr-board set reviewer.agent claude/opus --repo my-app  one repo
herdr-board delegate 12 --worker.agent pi/anthropic/claude-opus-5-5 --worker.effort medium   one task
herdr-board set worker.fallback "codex, claude/sonnet"
herdr-board coordinator new --agent claude/sonnet --effort low   the next Coordinator only
```

Or ask the Coordinator in your own words ("do this one with Opus on high effort", "let Codex review it"): it looks the
names up with `herdr-board models` and sets them the same way. It writes only a name from that list ("sol 6.1" is
`gpt-6.1-sol`, or it asks you), and when you name no model it gives none: the program then uses its own default. Where a
value comes from when several places set it: [settings](settings.md).

## A model

The model travels with its program, so a Claude model is never handed to Codex. The name is **the program's own**, and
each program names its models its own way:

| Program | Written as | Its names |
|---|---|---|
| `claude` | `claude/opus`, `claude/claude-opus-5-5`, `claude/claude-opus-5-5[1m]` | a short name (`default`, `fable`, `opus`, `sonnet`, `haiku`, `opusplan`: each the newest of its kind) or one of its full names (`herdr-board models claude`) |
| `codex` | `codex/gpt-6.1-sol` | what Codex lists for the account it is signed in with (its app-server's `model/list`; `codex debug models` when that cannot be asked) |
| `pi` | `pi/anthropic/claude-opus-5-5` (its provider first), `pi/anthropic/claude-opus-5-5:high` | what `pi --list-models` lists |
| `cursor` | `cursor/claude-opus-5-5-high`, `cursor/claude-opus-4-8[effort=high]` | what `cursor-agent models` lists; a listed model's name without its last part takes parameters in brackets |
| `opencode` | `opencode/anthropic/claude-opus-5-5` (its provider first) | what `opencode models` lists |
| `droid`, `mastracode` | no model | chosen in the program's own settings |
| the others | `agy/gemini-3-pro` | not checked: the names the program's own help gives |

OpenCode 2's full-screen app takes no `-m` (it quits on it), so the board first makes its conversation on the model,
in the folder it starts in (`opencode api session.create`), then opens that one: `opencode --auto --session <id>`.
A conversation it reopens keeps the model it has. OpenCode 1 is given `-m <model>`, as before (#394).

### A wrong name is refused when you set it

Not found out later, when the agent fails to start. The answer says what the program takes, and the nearest name:

```
$ herdr-board set worker.agent codex/sol-6.1
worker.agent: codex has no model "sol-6.1" for your ChatGPT sign-in (pro plan). Did you mean gpt-6.1-sol? It takes
gpt-6.1-sol, gpt-6-astra, gpt-6-sol, … (codex's own list for that sign-in). Write it after the program: codex/gpt-6.1-sol.
```

- **The list is asked of the program**, with the account you are logged in with, and kept for an hour. A name that is not
  on it is asked again first, so a model the program has just added is taken at once.
- **Codex's names depend on how you signed in.** A ChatGPT sign-in takes other names than an API key, and a name that is
  fine elsewhere is refused by Codex at the first turn ("The 'sol-6.1' model is not supported when using Codex with a
  ChatGPT account"). So the board asks Codex itself, through its app-server, which names this sign-in can use and which
  sign-in it is (`herdr-board models codex` says it). Where the app-server cannot be asked (an older Codex), it reads
  `codex debug models`.
- pi, cursor and opencode list only the models their own sign-ins and keys reach, so their lists are already yours.
- **When the list cannot be read** (the program is installed but not logged in, offline, or its list command fails), the
  name is refused too, with what the program said. Run its list command in a terminal to fix that, or leave the model
  out.
- **Claude Code has no command that lists its models**, so its names are written in the board's table of programs. A
  Claude model newer than that table is refused until the board is updated. Its short name (`claude/opus`) always means
  the newest of its kind.

### A model refused while the agent works

A name can still be refused when the agent runs: one written in a settings file by hand (only a value being set is
checked), a sign-in that changed, a program on another machine, or the program's own default model. The board catches
that at the end of the turn and never leaves the task in the agent with only its error:

- **It gave the program that model:** the same program starts again at once on its own default model, in the same
  worktree, told what was done so far. The row says so: `gpt-6.1 can't be used with your ChatGPT sign-in: started again on
  codex's own default model`, and the task panel lists it under "Took over". The reviewer the same: the review starts
  again on its program's default model.
- **The default model is refused too, or it gave none:** the task waits for you, with the reason and a name to try
  (`Agent stopped: gpt-6.1 can't be used with your ChatGPT sign-in; try gpt-6.1-sol`) and the next step in the panel:
  choose one for the task (`herdr-board set worker.agent codex/gpt-6.1-sol --task <id>`), or fix the program's own
  default (Codex: `model` in `~/.codex/config.toml`), then `Ctrl+L` (retry). The program is ended, so the retry starts it
  again on the model as it is set then.
- With a `worker.fallback` (or `reviewer.fallback`), the next on that list comes after the default model
  ([fallback](fallback.md)).

**A name is not checked** (it goes to the program as you wrote it) when:

- the program has no list and no names written in the table;
- the program is not installed here;
- the program is pointed at a provider of its own: Claude Code with `ANTHROPIC_BASE_URL` or a cloud
  (`CLAUDE_CODE_USE_BEDROCK`, `…_VERTEX`, `…_FOUNDRY`), or Codex with a `model_provider` in its `config.toml`.

## An effort

`<role>.effort` is how hard the agent thinks: `default` (the program's own) or a level, such as `low`, `medium` or `high`.

- **Each program has its own levels, and the board asks the program for them**, as it does for model names:
  - Codex: what `codex debug models` lists for each model (today `low` to `max`, and `ultra` on some). A level is checked
    against the model you chose (`codex/gpt-6-luna` has no `ultra`); with no model, against every level any model takes.
  - The others: what their `--help` lists at their effort argument (`claude --help`: `low` to `max`; `pi --help`: `off`
    to `max`).
  - The levels are kept for an hour. A level that is not on the list is asked again first, so a level the program has
    just added is taken at once.
  - **When they cannot be read** (the program is not installed here, is offline, or its help names none), the levels
    written in the board's table of programs are used ([the table in agents](agents.md#the-programs)). Never a refusal
    of every level.
- A level the role's program (or its model) does not have is refused, with the ones it has:

  ```
  $ herdr-board set worker.effort minimal
  worker.effort: codex has no effort "minimal": it takes low, medium, high, xhigh, max, ultra, as `codex debug models` lists them (or default, for its own).
  ```

- A program that takes no effort at start takes only `default`.
- Cursor takes its effort in the model's name (`cursor/claude-opus-4-8[effort=high]`). pi also takes one after its model
  (`:high`).
- An effort set for one program, and kept when you choose another (program or model), is left out at start if the new
  one does not have it. A start goes by the levels the program said last: it never waits to ask.
- `<role>.fast` is Codex's faster service tier. Elsewhere it does nothing.

## Catalog and routing

**A task can be given an entry of a catalog of models, picked for it before it starts: a cheap one for a typo, a strong
one for a hard refactor.** The entry is the task's program, model and effort for the whole of an agent's session; when
its work fails review, the next entry of the catalog takes the task over, in the same worktree. Trivial work stops
paying for the strongest model, and hard work gets more effort.

```
herdr-board models catalog            the catalog as the board reads it: each entry, its cost, how it fared lately
herdr-board models catalog --init     write the board's example to ~/.herdr-board/models.toml, to change
herdr-board models stats [--since 30d]   routed tasks by entry: tokens, cost, reviews failed, escalations
herdr-board set route.mode coordinator   the Coordinator routes each task it adds or delegates
```

### The catalog

`~/.herdr-board/models.toml` is yours. Until it is there the board's example is the catalog (three Claude entries,
`leve`, `padrao` and `pesado`, with entries for other programs left in comments to adjust to what you have). Each
`[[entry]]`:

| key | what it is |
|---|---|
| `name` | one word, what `--route` names |
| `agent` | a program with an optional model after a slash, as `worker.agent` takes it (`claude/sonnet`) |
| `effort` | one its program takes, or `default` |
| `cost` | its relative cost, on any scale (1, 3, 10) |
| `good_for`, `avoid_for` | free words, in any language: what the Coordinator reads to choose it, or not |
| `escalate_to` | the entry that takes over when this one's work fails review, or it cannot go on |
| `max_files`, `max_minutes` | optional: roughly how big a task it is for, a sign for the Coordinator |

An entry is checked as `set worker.agent` checks a value: a program that is not one, a model its program does not take,
an effort it does not have, an unknown key, an `escalate_to` to no entry or in a circle. An entry that fails is named,
with why, by `models catalog`, and is not used.

**`allow.agents`** (`[allow] agents = ["claude"]`) is the list of programs allowed here. Without it every program is.
With it, any other is refused wherever it is named: an entry, a setting (`set worker.agent codex`), a fallback,
`attempts.by`, `--worker.agent`. A value a hand wrote in your settings that names one does not count (`show` says so),
and a built-in default that names one, such as `reviewer.agent codex`, is not started: `show` marks it
(`refused when it starts: …`), and the review fails with the same words until you set another.

### Routing: `route.mode`

`none` (the default): nothing is routed; every task works on `worker.agent`, and the Coordinator never chooses a model
of its own.

`coordinator`: for a task the Coordinator adds or delegates with no model named by you, it reads the catalog
(`models catalog`, with each entry's last 30 days beside it) and chooses one entry by its `good_for` and `avoid_for`,
with the reason in a line: `board add "…" --route padrao --route.reason "refator pequeno, 2 arquivos"`. It never
leaves the catalog. The router is the Coordinator itself: no extra model is called.

The board makes the entry the task's settings, as a place of their own just under the task's own (`show <id>` says
where each came from):

```
  worker.agent     claude/sonnet  routed: padrao — "refator pequeno, 2 arquivos"
  worker.effort    medium         routed: padrao — "refator pequeno, 2 arquivos"
  worker.fallback  claude/opus    routed: padrao — "refator pequeno, 2 arquivos"
```

- **What you name wins.** "Com opus" is `--worker.agent claude/opus`, the task's own setting: the route is then not read
  at all (its effort neither), and `--route` given with it says it was not used.
- **The plan and the review are not routed.** They stay on `plan.agent` and `reviewer.agent`: the strong check is what
  makes a cheap worker safe.
- **Tasks the board starts by itself** (an automation, autopilot, a task chained with `--after`) get `route.default`,
  an entry or `none`, under `route.mode coordinator`: no model is asked. An automation can name its own:
  `automations add … --route leve` (its `route.default`), which counts whatever `route.mode` says.
  A coordinator on autopilot may pass `--route` only where its `autopilot.mayChange` allows `worker.agent`: a route
  sets the task's program; otherwise its answer says it was not applied, and `route.default` is used.
- The model stays the same for the whole of an agent's session: changing it midway would lose its context.

### Escalation

A routed task's `worker.fallback` is the chain of its entry's `escalate_to`. The next entry takes over as any fallback
does ([the fallback agent](fallback.md)): in the same worktree and pane, with the task, its commits, the last words of
the one before and the open findings, never in the middle of a session. With its own effort.

It happens earlier than for other tasks: after `route.escalateAfter` failed reviews (1 by default), not after
`review.rounds`. The last entry of the chain has the usual `review.rounds`. A provider that is down, a lost login or no
allowance hand it over too, as for any task. A new delegation starts on the first entry again.

### Measuring

For each routed task the board keeps the entry chosen, the reason, who chose it, and each escalation (`details <id>`,
`route`); its tokens are counted as for `limit.tokens`, whether or not a limit is set. `models stats` adds them up by
the entry first chosen: how many tasks, tokens per task, the entry's cost and the cost in all, the share whose review
failed, the share that escalated, and the repos where an entry escalated most (`leve: escalated 40% in web`). The
catalog shows the Coordinator a line of it per entry, to choose better.

## A new program

How a program's models are listed and read is one entry in its row of the table of programs
(`crates/core/src/agents/kinds/`, `models`): the command that lists them and its format (and Codex's app-server, `server`),
or its short and full names. A
program with neither is not checked. Its effort levels are read from its `--help` unless its row says otherwise
(`efforts_read`: Codex's catalog, or only the table); its `efforts` are the levels used when they cannot be read.
