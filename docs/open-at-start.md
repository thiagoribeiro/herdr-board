# The board at herdr's start

**When herdr starts, the board is there, once.** herdr's startup hook (`[[startup]]` in `herdr-plugin.toml`) starts
`herdr-board ensure-open`, a process of its own, so herdr's start waits for nothing. It also opens with no tasks at
all, and it asks no agent anything: only herdr.

- **One board, never two.** It keeps the board that runs (its note in the board's data folder, its process alive), or
  a board pane herdr has where the board's program runs (one another call just opened, before it has said where it
  is). The look and the open are one step under a lock per herdr session, so two starts at once open one board. Only
  when there is none does it open one, in a tab of its own named `board`, as the board key does.
- **The tab herdr brings back is no board.** After a restart herdr brings the board's tab back with its session, as
  the pane `Work board`, but with a bare shell in it, not the board. A new board opens, and that shell, when nothing
  runs in it, is closed: one board tab, not a board and a stray shell. One where something runs stays.
- **It never closes a board** and never takes you back to where you came from: that is the board key's toggle, not
  this.
- **After a crash** the note the dead board left is no board: a board herdr brought back is kept, else a new one opens.
- **It waits for herdr's socket and its first space** (a minute at most), then a moment for a board herdr brings back
  to come up. herdr opens no tab before it has a space: a server started with no client (`herdr server`) has none, and
  there the board opens with the key once you are in.

## Who gets the keyboard

**Only a board it opened itself, at a fresh start, and only when you did not move meanwhile.** With
`board.focusOnStartup` on (the default), a new board takes the keyboard when herdr answered at once and the focused
pane stayed the same while the board waited. A board that was there already (herdr kept or brought it back: a reload,
a handoff) stays as it is, and so does the keyboard. With `board.focusOnStartup` off, the board opens behind: herdr
gives a new tab the keyboard, and it goes back to the pane that had it. Except where it was on the shell herdr brought
back in the board's place: there the new board takes it, as that shell goes.

## Turning it off

| Setting | Default | |
|---|---|---|
| `board.openOnStartup` | `true` | `herdr-board set board.openOnStartup false`: herdr's start opens nothing; the board key still does |
| `board.focusOnStartup` | `true` | `herdr-board set board.focusOnStartup false`: it opens, and the keyboard stays where herdr put it |

With `board.open` set to `popup`, herdr's start opens nothing: the popup is the key's only ([the board](board.md)).

## The key that brings it back

The board key is herdr's: a `[[keys.command]]` in herdr's `config.toml` that runs the plugin's `open` action, offered
at the [first run](install.md) as `prefix+a`. From anywhere in herdr it goes to the board that runs (the one opened at
the start too), and pressed on the board it takes you back. It is not one of the board's own keys, so `board.keys`
cannot move it: change the `key` line in herdr's config and run `herdr server reload-config`
([adding the blocks by hand](install.md#adding-the-blocks-by-hand)).

The code is `crates/board/src/board/run/ensure.rs`.
