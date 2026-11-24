# Status

**`herdr-board status` says in one line per task where every active task stands, and whose move it is.** Nothing is
asked of a model: it reads the board's own records, as the board's rows do.

```
$ herdr-board status
BRD-1     Finish the review page          ready to approve 2h   you     approve PR#64 · Pass · 1 minor
BRD-3     Sidebar tree                    working          14h  you     No change for 14h · Finishing
BRD-7     Dark mode                       to do            1d   you     delegate it
TRM-4     Export to CSV                   working          3m   github  Waiting for GitHub checks · 0 of 1 passed
TRM-5     Faster import                   needs you        10m  you     Pull request conflicts with main · …
```

Each line has:

| Column | What it is |
|---|---|
| the task | its key and number, and its title, shortened |
| where it stands | the words of the board's list: `working`, `needs you`, `ready to approve`, `behind main`, `queued` … |
| for how long | since it came to where it stands (a working task: since its agent was set going) |
| whose turn | `you`, `agent` (its agent, or its reviewer), `board` (the board's own steps, a task waiting for a slot, and a retry it does by itself) or `github` (a merge that waits for the pull request's checks). It is herdr's sidebar's own answer, with three more: a stalled task is yours to look at, a merge waiting for GitHub's checks is GitHub's, and a task nobody started is yours to delegate |
| the next step | one sentence: what to approve, what it waits for, or why it stalled ([troubleshooting](troubleshooting.md#a-task-is-stalled)) |

Done and archived tasks are left out; `--all` lists them too.

## For scripts

`herdr-board status --json` prints the same as a list, one object per task:

| Field | What it is |
|---|---|
| `id` | the task's id (`herdr-board/3`) |
| `title`, `stage` | its title and the board's stage for it |
| `state` | where it stands, in the list's words |
| `since`, `for` | since when (milliseconds since 1970), and how long, in words (`14h`) |
| `whose` | `you`, `agent`, `board` or `github` |
| `next` | the next step |
| `stall` | `{ "event", "why" }` when it is stalled (`stalled`, `conflict`, `unreachable`), else `null` |
