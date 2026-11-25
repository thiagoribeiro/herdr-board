# Your diff tool

**See what a task changed in the tool you already use, without leaving herdr.** `Alt+G` on a task opens a herdr pane
beside the board, in that task's folder (its worktree), and runs your diff tool there. Quit the tool and the pane
closes: you are back on the board, where you were.

## What it runs

The setting `review.tool` (yours, as `board.*` settings are) is the command. Built in it is `git diff {from}`.
`{from}` is the commit to compare with:

- **since your approval**: the commit you last said yes to (`Ctrl+L`), when the branch moved on since. An approved
  merge that comes back to you because something changed (it says "Changed since your approval") shows only that.
- **against main** otherwise: where the task left main (the main commit last merged into it, else the one it started
  from), so the diff is the task's own work, uncommitted changes too.

Some choices:

```
herdr-board set review.tool lazygit
herdr-board set review.tool 'git diff {from} | delta'
herdr-board set review.tool 'nvim -c "DiffviewOpen {from}"'
```

A command without `{from}` runs as it is. The pane runs it at its own shell with `; exit` after it, so a tool that
waits for you (a pager, lazygit) keeps the pane open until you quit it, and one with nothing to show closes it at once.

## Where it works

In herdr, on a task of this machine. In the board's tab the pane is split off the board's own pane; with the board as a
popup (`board.open: popup`) it is split off the pane you came from, and the popup closes. On the web board and on
another machine's task the key only says so.
