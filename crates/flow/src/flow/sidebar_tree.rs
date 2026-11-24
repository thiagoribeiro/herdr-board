//! The tree of herdr's sidebar: each task's space under the coordinator that added it, and a task that waits for
//! another (`--after`) under that one. herdr 0.9.5 nests nothing, so the tree is two things: a prefix before the task's
//! number in its space's name (`├─ `, `└─ `, `│  `, three cells a level, four levels at most), and the order of the
//! spaces, each coordinator's tasks right after the space its tab is in (herdr's agents list, grouped by space, follows
//! that order). Tasks you added yourself (or an automation did) stand outside every tree, last: no prefix, except under
//! a task they wait for. Only while herdr groups its agents by space: in its `priority` sort no tree is drawn.
use super::sidebar_entry::cut;
use crate::coordinator::co::Co;
use crate::json::*;
use std::collections::HashMap;

pub const BRANCH: &str = "├─ ";
pub const LAST: &str = "└─ ";
pub const GOES_ON: &str = "│  ";
pub const ENDED: &str = "   ";
/// Levels drawn at most: a task deeper than that stands at the last of them, so that the number stays inside herdr's
/// 26 cells (12 cells of tree, the dot and a number like `BRD-123`).
pub const DEEPEST: usize = 4;
/// The group of tasks you added: no coordinator.
pub const YOURS: &str = "";

/// One task's place: its id, its space, the coordinator it is under (`main`, `machine`, `project:<repo>`, or none),
/// and the prefix before its number.
#[derive(Clone, Debug, PartialEq)]
pub struct Place { pub id: String, pub space: String, pub group: String, pub prefix: String }

/// The coordinator that added a task, by its key: the one you asked, or whose autopilot did. None: you, or an automation.
pub fn group_of(t: &Obj) -> &str { t.s("coordinator") }
/// Groups in their order: the Coordinator's, the machine's, the projects' by name, yours last.
fn rank(g: &str) -> (u8, &str) { match g { "main" => (0, ""), "machine" => (1, ""), YOURS => (3, ""), p => (2, p) } }

/// `tasks`: (task, its space) for each task with a space in the sidebar, in the board's order. Out: the same tasks in
/// the sidebar's order, each with its prefix.
pub fn tree(tasks: &[(&Obj, &str)]) -> Vec<Place> {
    let mut groups: Vec<&str> = tasks.iter().map(|(t, _)| group_of(t)).collect();
    groups.sort_by_key(|g| rank(g));
    groups.dedup();
    let mut out = Vec::with_capacity(tasks.len());
    for g in groups {
        let mine: Vec<(&Obj, &str)> = tasks.iter().filter(|(t, _)| group_of(t) == g).copied().collect();
        // Under the first task it waits for that is in the same tree, else at the top of it: each task's parent, once.
        let parent: Vec<Option<usize>> = mine.iter().map(|(t, _)| crate::after::after_of(t).iter().filter(|a| a.as_str() != t.s("id")).find_map(|a| mine.iter().position(|(o, _)| o.s("id") == a.as_str()))).collect();
        let mut kids: HashMap<Option<usize>, Vec<usize>> = HashMap::new();
        for (i, p) in parent.iter().enumerate() { kids.entry(*p).or_default().push(i); }
        let w = Walk { mine: &mine, kids: &kids, group: g, drawn: g != YOURS };
        let mut seen = vec![false; mine.len()];
        let top = kids.get(&None).cloned().unwrap_or_default();
        w.walk(&top, "", 0, &mut seen, &mut out);
        // Waits in a circle (refused on `after`, but a task's list can be edited): what was never reached goes at the top.
        let rest: Vec<usize> = (0..mine.len()).filter(|i| !seen[*i]).collect();
        Walk { kids: &HashMap::new(), ..w }.walk(&rest, "", 0, &mut seen, &mut out);
    }
    out
}
struct Walk<'a> { mine: &'a [(&'a Obj, &'a str)], kids: &'a HashMap<Option<usize>, Vec<usize>>, group: &'a str, drawn: bool }
impl Walk<'_> {
    fn walk(&self, list: &[usize], lead: &str, depth: usize, seen: &mut [bool], out: &mut Vec<Place>) {
        let list: Vec<usize> = list.iter().copied().filter(|i| !seen[*i]).collect();
        for (n, &i) in list.iter().enumerate() {
            if seen[i] { continue; }
            seen[i] = true;
            let last = n + 1 == list.len();
            // Your own tasks' tops are not drawn; their kids are, one level each.
            let drawn = self.drawn || depth > 0;
            let prefix = if drawn { format!("{lead}{}", if last { LAST } else { BRANCH }) } else { String::new() };
            let (t, space) = self.mine[i];
            out.push(Place { id: t.t("id"), space: space.to_string(), group: self.group.to_string(), prefix });
            // Past the deepest level a task's kids stand at its own level.
            let next = if !drawn { String::new() } else if lead.chars().count() / 3 + 1 >= DEEPEST { lead.to_string() } else { format!("{lead}{}", if last { ENDED } else { GOES_ON }) };
            if let Some(k) = self.kids.get(&Some(i)) { self.walk(k, &next, depth + 1, seen, out); }
        }
    }
}

/// Does herdr list its agents space by space (`[ui] agent_panel_sort`: "spaces", its default, or "workspaces")? In
/// its "priority" sort the order of the spaces is not the list's, and a tree would stand in pieces.
pub fn grouped() -> bool {
    let conf = crate::board::theme::read_config();
    let sort = conf.get("ui").and_then(|u| u.as_table()).and_then(|u| u.get("agent_panel_sort")).and_then(|s| s.as_str()).unwrap_or("spaces");
    sort != "priority"
}

/// The space each live coordinator's tab is in, by its key: where its tasks go after.
pub fn anchors(socket: &str) -> Vec<(String, String)> {
    crate::coordinator::state::all(socket).into_iter().filter_map(|co: Co| crate::coordinator::state::get(&co).filter(|c| c.b("pane") && c.b("space")).map(|c| (co.settings_key(), c.t("space")))).collect()
}

/// herdr's spaces (`have`, in its order) in the tree's order: every other space keeps its place, each coordinator's
/// tasks come right after its space, and the rest (yours, and those of a coordinator not in herdr now) last.
pub fn order(have: &[String], places: &[Place], anchors: &[(String, String)]) -> Vec<String> {
    let ours: Vec<&String> = places.iter().map(|p| &p.space).collect();
    let anchored = |p: &Place| anchors.iter().find(|(g, _)| *g == p.group).map(|(_, s)| s.clone()).filter(|s| have.contains(s) && !ours.contains(&s));
    let mut out: Vec<String> = vec![];
    for s in have.iter().filter(|s| !ours.contains(s)) {
        out.push(s.clone());
        out.extend(places.iter().filter(|p| anchored(p).as_ref() == Some(s) && have.contains(&p.space)).map(|p| p.space.clone()));
    }
    out.extend(places.iter().filter(|p| anchored(p).is_none() && have.contains(&p.space)).map(|p| p.space.clone()));
    out.dedup();
    out
}
/// A space's name: its prefix, then the task's number and title, cut (with `…`) where herdr would cut it.
pub fn labelled(prefix: &str, name: &str) -> String { cut(&format!("{prefix}{name}")) }
