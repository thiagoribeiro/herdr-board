# Your diff tool

**See what a task changed in the tool you already use, without leaving herdr.** `Alt+G` on a task opens a herdr pane
beside the board, in that task's folder (its worktree), and runs your diff tool there. Quit the tool and the pane
closes: you are back on the board, where you were.

## What it runs

The setting `board.diffTool` (yours only, as every `board.*` setting) is the command. Built in it is
`git diff --color {from} | less -+F -R`: git's diff in `less`, which waits for your `q` also when the diff is short
(`-+F` takes back the quit-at-once that git's own `LESS=FRX` would give it). `{from}` is what to compare with:

- **since your approval**: when an approved merge came back to you because the branch changed (it says "Changed since
  your approval"), the commit you approved. Main merged into the branch since is merged into that commit too (a tree
  made with `git merge-tree`, which `git diff` takes as it takes a commit), so only the task's own change since your
  yes shows. A new yes, or a fresh start of the task, ends it.
- **against main** otherwise, and where merging main into the approved commit conflicts: where the task left main (the
  main commit last merged into it, else the one it started from), so the diff is the task's own work, uncommitted
  changes too.

Some choices:

```
herdr-board set board.diffTool lazygit
herdr-board set board.diffTool 'git diff {from} | delta --paging always'
herdr-board set board.diffTool 'nvim -c "DiffviewOpen {from}"'
```

A command without `{from}` runs as it is. The pane runs it at its own shell with `; exit` after it, so it closes when
the command ends: a tool that waits for you (lazygit, a pager told to wait) keeps it open until you quit, and a pager
that quits by itself on a short diff (`less -F`, delta's default paging) closes it at once.

## Where it works

In herdr, on a task of this machine. In the board's tab the pane is split off the board's own pane; with the board as a
popup (`board.open: popup`) it is split off the pane you came from, and the popup closes. On the web board and on
another machine's task the key only says so.
