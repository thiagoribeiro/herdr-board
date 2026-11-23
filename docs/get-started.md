# Get started

You need [Herdr](https://herdr.dev) 0.9.3+ on macOS or Linux, and Claude Code, Codex, pi or another agent herdr supports.

## Option A: do it yourself

1. Install:

   ```sh
   herdr plugin install thiagoribeiro/herdr-board
   ```

2. In Herdr, run the action **Open the work board**. Say yes to the key it offers (`prefix+a`).
3. Optional: add `~/.herdr-board/data/bin` to your PATH so you can type `herdr-board`.

## Option B: let an agent do it

Paste this into Claude Code, Codex or pi, running inside Herdr:

```text
Install the herdr-board plugin for Herdr and set it up for me.
1. Check `herdr --version` is 0.9.3 or newer, and that git and curl (or wget) exist. Stop and tell me if not.
2. Run `herdr plugin install thiagoribeiro/herdr-board`.
3. Find the plugin's folder with `herdr plugin list` and run `sh <that folder>/bin/run.sh install`.
4. Add ~/.herdr-board/data/bin to my PATH in my shell's start-up file, so `herdr-board` works in new shells.
5. Run `herdr-board doctor` and fix anything it reports.
6. Open the board: `herdr plugin action invoke open --plugin herdr-board`.
Then tell me in two lines: it is ready, and to say yes to the key it offers (prefix+a) on its first screen.
Details if you need them: https://github.com/thiagoribeiro/herdr-board/blob/main/docs/install.md
```

## Start it

- Press `prefix+a` in Herdr, or type `herdr-board` in any pane.
- Type what needs doing and press `Enter`. The Coordinator makes it a task.
- Press `Ctrl+S` instead of `Enter` to put an agent on it right away.
- Press `?` to see everything else.

Update later with `herdr-board update`.

More detail (building from source, older installs, uninstalling): [installing](install.md).
