# What needs you, one key at a time

**`Ctrl+Q` goes to the next task that needs you and puts the reply box on it.** What you type next is your answer to
that task's agent; `Enter` sends it. Press it again for the one after. Answering everything that waits takes this key,
your words and `Enter`, once per task.

```
   ▾ needs you
▌  ? #9   Settings file cleanup                                                      14m ago
▌         Asks you · Should the old settings file be removed, or kept for one more version?
   @ #10  Release notes                                                               3m ago
          Needs a login
   ! #12  Add an undo for clear                                          R#2 PR#18   12m ago
          Review failed · 1 critical, 1 major
╭─ this machine › proj ›  #9  · Agent ──────────────────────────────────────────────────────╮
│                                                                                            │
│  ❯ Keep it                                                                                 │
│                                                                                            │
╰────────────────────────────────────────────────────────────────────────────────────────────╯
  enter send to #9's agent · shift+enter new line · esc put away
```

## Where it goes

- **In the order of the list**, from the row you are on, down and round again to the top. Off a row that needs you,
  it goes to the next one below, else the first.
- **The selected task first.** On a task that needs you but without the reply box on it, the key puts the box there.
  With the box already on it, the key goes on to the next one.
- **Only what the list shows.** A filter (`Tab`) or a folded heading keeps what it hides. Under `Working`, the key says
  `Nothing in view needs you.` and moves nothing. Under `Needs you`, only the tasks that wait are listed, so the key goes
  through all of them.
- **Only with the text box empty.** With words typed, the key belongs to the text box, like every other key. Your draft
  and the task it is for stay as they are.

## After an answer

By default the board stays on the task you answered: its row shows the answer on its way, and `Ctrl+Q` goes on when
you want. With `board.nextAfterReply` on, an answer sent to a task that needs you moves the board to the next one
itself, with the reply box on it:

```
herdr-board set board.nextAfterReply true
```

An answer to a task that does not wait for you (a working agent, a task to do) never moves the board.

## What its row says

A task under `needs you` has two lines. The second one says why it waits first, then the task's own words:

| It waits for | Its second line starts with |
|---|---|
| an answer to a question | `Asks you` |
| a yes or no | `Asks for your OK` |
| a sign-in | `Needs a login` |
| your call on work whose review failed | `Review failed` |
| a plan to approve | `Plan ready` |
| one of several attempts to keep ([attempts](attempts.md)) | `Pick one` |

A task that went wrong (`✗`, or `!` on a merge or a pull request that failed) has no reason word: its mark says it
already, and its line says what failed. A task with nothing to say but that it needs you reads `Needs you`.

The time on the right is **how long it has waited**: from the moment its agent stopped to wait for you, as herdr saw
it. A task that waits because of a step of the board (a review that failed, a plan, a merge) counts from that step.

## What stays as it was

- `Space` still talks to the selected row's agent, and back to the Coordinator.
- `Alt+O` still goes into its agent's pane.
- `Tab` still goes through the filters, and nothing jumps the filter for you.
- `Ctrl+L` still says yes to whatever the task waits for.
- **Neither the task you answer nor your draft ever moves when the list reorders.** A task that starts working again
  moves to another group, but the selection and the reply box stay on the task you are typing to. If that task goes
  away (archived on another board, say), your words are put away, nothing is sent, and `Ctrl+Z` brings them back.

All the keys: [keys](keys.md). The rows and their marks: [the board](board.md#a-task-that-needs-you).
