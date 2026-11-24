# Changelog

## Unreleased

### Herdr's sidebar

- Task agents stand in a tree under the coordinator that added them (the Coordinator, the machine's or a project's):
  `├─ BRD-2 …`, `└─ BRD-3 …`, and a task that waits for another (`--after`) one level under that one. The reviewer's
  entry is in its task's space, right under the task's agent. Tasks you added yourself come last, outside every tree.
  The tree is a prefix before the task's number in its space's name, and the order of the spaces: Herdr 0.9.5 nests
  nothing itself. It is drawn while Herdr sorts its agents list by space (`agent_panel_sort = "spaces"`, Herdr's
  default); in its `priority` sort the spaces keep their plain names and order, since a tree would stand in pieces
  there ([herdr's sidebar](docs/sidebar.md#the-tree-tasks-under-their-coordinator)).
