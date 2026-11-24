//! The tree of herdr's sidebar: each task's space under the coordinator that added it, and a task that waits for
//! another (`--after`) under that one. herdr 0.9.5 nests nothing, so the tree is two things: a prefix before the task's
//! number in its space's name (`├─ `, `└─ `, `│  `, three cells a level), and the order of the spaces, each
//! coordinator's tasks right after the space its tab is in (herdr's agents list, grouped by space, follows that order).
//! Tasks you added yourself (or an automation did) stand outside every tree, last: no prefix, except under a task they
//! wait for. The order is put once per change of the tree, not each tick, so a space you drag stays where you put it.
use crate::coordinator::co::Co;
use crate::json::*;
use super::sidebar_entry::CAP;

pub const BRANCH: &str = "├─ ";
pub const LAST: &str = "└─ ";
pub const GOES_ON: &str = "│  ";
pub const ENDED: &str = "   ";
/// The group of tasks you added: no coordinator.
pub const YOURS: &str = "";

/// One task's place: its id, its space, the coordinator it is under (`main`, `machine`, `project:<repo>`, or none),
/// and the prefix before its number.
#[derive(Clone, Debug, PartialEq)]
pub struct Place { pub id: String, pub space: String, pub group: String, pub prefix: String }

/// The coordinator that added a task, by its key: the one you asked, or whose autopilot did. None: you, or an automation.
pub fn group_of(t: &Obj) -> String { t.t("coordinator") }
/// Groups in their order: the Coordinator's, the machine's, the projects' by name, yours last.
fn rank(g: &str) -> (u8, String) { match g { "main" => (0, String::new()), "machine" => (1, String::new()), YOURS => (3, String::new()), p => (2, p.to_string()) } }

/// `tasks`: (task, its space) for each task with a space in the sidebar, in the board's order. Out: the same tasks in
/// the sidebar's order, each with its prefix.
pub fn tree(tasks: &[(Obj, String)]) -> Vec<Place> {
    let mut groups: Vec<String> = tasks.iter().map(|(t, _)| group_of(t)).collect();
    groups.sort_by_key(|g| rank(g));
    groups.dedup();
    let mut out = vec![];
    for g in groups {
        let mine: Vec<&(Obj, String)> = tasks.iter().filter(|(t, _)| group_of(t) == g).collect();
        let ids: Vec<String> = mine.iter().map(|(t, _)| t.t("id")).collect();
        // Under the first task it waits for that is in the same tree, else at the top of it.
        let parent = |t: &Obj| crate::after::after_of(t).into_iter().find(|a| *a != t.s("id") && ids.contains(a)).unwrap_or_default();
        let kids = |of: &str| mine.iter().filter(|(t, _)| parent(t) == of).map(|x| (*x).clone()).collect::<Vec<_>>();
        let top = kids("");
        // Waits in a circle (refused on `after`, but a task's list can be edited): what was never reached goes at the top.
        let mut seen: Vec<String> = vec![];
        let under_coordinator = g != YOURS;
        walk(&top, "", under_coordinator, &g, &kids, &mut seen, &mut out);
        let rest: Vec<(Obj, String)> = mine.iter().filter(|(t, _)| !seen.contains(&t.t("id"))).map(|x| (*x).clone()).collect();
        walk(&rest, "", under_coordinator, &g, &|_: &str| vec![], &mut seen, &mut out);
    }
    out
}
fn walk(list: &[(Obj, String)], lead: &str, drawn: bool, g: &str, kids: &dyn Fn(&str) -> Vec<(Obj, String)>, seen: &mut Vec<String>, out: &mut Vec<Place>) {
    for (i, (t, space)) in list.iter().enumerate() {
        let id = t.t("id");
        if seen.contains(&id) { continue; }
        seen.push(id.clone());
        let last = i + 1 == list.len();
        let prefix = if drawn { format!("{lead}{}", if last { LAST } else { BRANCH }) } else { String::new() };
        out.push(Place { id: id.clone(), space: space.clone(), group: g.to_string(), prefix });
        let next = if drawn { format!("{lead}{}", if last { ENDED } else { GOES_ON }) } else { String::new() };
        walk(&kids(&id), &next, true, g, kids, seen, out);
    }
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
pub fn labelled(prefix: &str, name: &str) -> String {
    let all: Vec<char> = format!("{prefix}{name}").chars().collect();
    if all.len() <= CAP { all.into_iter().collect() } else { format!("{}…", all[..CAP - 1].iter().collect::<String>().trim_end()) }
}
