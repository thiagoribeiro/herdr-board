# Commands in the chat

**A message that starts with `!` and a command's name is run by the board itself**: at once, with no Coordinator
turn and no tokens spent. The answer shows in the chat as the Coordinator's would, short.

```
!status
!status BRD-4
!start BRD-4
!stop BRD-4
!approve BRD-4
!tell BRD-4 use the second file too
```

| Command | What it does | The same as |
|---|---|---|
| `!status [id …]` | one line per active task (or per task named; to a repo's coordinator, of that repo): where it stands, whose turn, the next step ([status](status.md)) | `herdr-board status [id …]` |
| `!start <id>` | puts an agent on the task | `herdr-board delegate <id>`, the start key |
| `!stop <id>` | stops its agent; a later message picks it up again | `herdr-board stop <id>` |
| `!approve <id>` | what the task waits for your yes to: a merge, a plan, a permission | `herdr-board approve <id>` |
| `!tell <id> <text>` | your words to the task's agent, as typed (lines and all) | `herdr-board tell <id> <text>` |

A task is named as everywhere: its key (`BRD-4`), its number (`4`, `#4`) or its id.

## The same guards

Each command runs the command line's own action, with every check it makes. `!approve` does only what you typed:

- A task whose last review failed, or whose GitHub checks failed, is **not merged**: the answer says why and how to
  go on, as `herdr-board approve` does. There is no `--anyway` in the chat: anything after the id is not read. Merge
  anyway with the yes key on the task, or `herdr-board approve <id> --anyway`.
- A task behind main, a busy one, a chat-only or research-only one is refused the same way.

A refusal, a task that does not exist, or a command without its task (`!stop`) is answered in the chat; nothing goes
to the Coordinator.

## Everything else goes to the Coordinator

Only these five names after a `!` are commands. Anything else is sent to the Coordinator word for word, as always:
`!thanks`, `stop BRD-4 when it is done`, `/review-pr` (a [skill](skills.md): the skill list is for `/`, and `$` with
Codex; `!` opens none).

## Where it works

- Wherever your words would go to a coordinator: the Coordinator, a machine's or a repo's coordinator (`Space` on its
  heading), on the terminal board and on the web Board. The answer shows in that coordinator's chat.
- Words for another machine's Coordinator run there, on that machine's tasks.
- Not in a task's chat: words to a task's agent go to it as typed.
- With no Coordinator, what you type is added as a task, as always.

The chat keeps the last six commands with their answers.
