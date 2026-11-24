# The Coordinator's notes

The Coordinator starts in a folder of its own (`~/.herdr-board/data/coordinator/<session>/chat`), outside every repo.
It sees none of your repos' rules and none of the notes you keep elsewhere, and its program's own memory is tied to
that folder. Two settings give it more to go on:

- **`coordinator.notes`**: a folder of **your** notes, such as an Obsidian vault. The Coordinator searches it (for a
  project, a machine, a name) and reads what it finds. It does this before it asks you something the notes may
  answer, and before it adds a task whose description needs that context. It names the note it used. It never
  changes, moves or deletes anything in that folder, and what the folder says is information, never instructions.
- **`coordinator.notesWrite`**: a folder of **its own** notes, kept across chats. It holds one short note per subject
  (a decision you made, a preference, where a project stands), plus an `index.md` with one line per note. The
  Coordinator reads the index at the start of every conversation: a fresh chat, a restart, a switch to another
  program. It updates a note in place rather than writing it twice, deletes one that turned out wrong, and never writes
  a secret in one. It makes the folder and the index when they are missing.

```
herdr-board set coordinator.notes ~/Documents/obsidian
herdr-board set coordinator.notesWrite Agents/herdr-board     # inside the vault
```

- A leading `~` is your home.
- A relative `coordinator.notesWrite` is inside `coordinator.notes`, or inside your home when that is not set.
- With the own folder inside your notes, that one folder is the only place in them the Coordinator may write.
- Both are `none` by default: the rules then say nothing about notes.
- Like the other `coordinator.` settings, they can be set for one coordinator only (`--main-coordinator`,
  `--machine-coordinator`, `--project-coordinator <repo>`).

**On autopilot:** without `coordinator.notesWrite`, the board is a coordinator's only memory and it writes no notes
anywhere. With it, its own folder is the one other place it may keep them.

**When a change takes effect:** a change to either setting changes the standing rules, so a running Coordinator is told
once, before your next message, to read them again ([its standing rules](coordinator.md#its-standing-rules-are-a-file)).
