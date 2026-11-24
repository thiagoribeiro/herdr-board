//! herdr's sidebar as a tree: each task's space under the coordinator that added it, a task that waits for another
//! under that one, your own tasks outside every tree and last; the prefix before the number (`├─ `, `└─ `, `│  `), the
//! spaces put in that order once per change of the tree (herdr's agents list, grouped by space, follows them), and
//! a task's reviewer in its task's space, so right under its agent.
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::coordinator::co::Co;
use sw::flow::sidebar::{sync_sidebar, tell_sidebar};
use sw::flow::sidebar_tree::{labelled, order, tree, Place};
use sw::herdr::Herdr;
use sw::json::*;
use sw::store::tasks::{update, Change};

fn t(id: &str, coordinator: &str, after: &[&str]) -> (Obj, String) {
    (obj(json!({ "id": id, "coordinator": coordinator, "after": after })), format!("w-{id}"))
}
fn prefixes(places: &[Place]) -> Vec<(String, String)> { places.iter().map(|p| (p.prefix.clone(), p.id.clone())).collect() }
fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> { list.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect() }

#[test]
fn the_tree_puts_each_coordinators_tasks_under_it_a_waiting_task_under_the_one_it_waits_for_and_yours_last_with_no_prefix() {
    let tasks = [
        t("p/1", "", &[]),
        t("p/2", "main", &[]),
        t("p/3", "main", &["p/2"]),
        t("p/4", "project:p", &[]),
        t("p/5", "main", &["p/3"]),
        t("p/6", "", &["p/1"]),
        t("p/7", "main", &[]),
        t("p/8", "machine", &[]),
        // waits for a task of another tree: at the top of its own
        t("p/9", "machine", &["p/2"]),
        t("p/10", "main", &["p/2"]),
    ];
    let places = tree(&tasks);
    assert_eq!(prefixes(&places), pairs(&[
        ("├─ ", "p/2"),
        ("│  ├─ ", "p/3"),
        ("│  │  └─ ", "p/5"),
        ("│  └─ ", "p/10"),
        ("└─ ", "p/7"),
        ("├─ ", "p/8"),
        ("└─ ", "p/9"),
        ("└─ ", "p/4"),
        ("", "p/1"),
        ("└─ ", "p/6"),
    ]));
    assert_eq!(places.iter().map(|p| p.group.as_str()).collect::<Vec<_>>(), ["main", "main", "main", "main", "main", "machine", "machine", "project:p", "", ""]);
    // A wait in a circle draws no loop: each task once.
    let circle = tree(&[t("p/1", "main", &["p/2"]), t("p/2", "main", &["p/1"])]);
    assert_eq!(circle.len(), 2, "{circle:?}");
}

#[test]
fn the_spaces_go_right_after_their_coordinators_space_every_other_space_keeps_its_place_and_the_rest_go_last() {
    let places = tree(&[t("p/1", "", &[]), t("p/2", "main", &[]), t("p/3", "project:p", &[]), t("p/4", "main", &["p/2"]), t("p/5", "project:q", &[])]);
    let have: Vec<String> = ["w-p/1", "home", "w-p/4", "mine", "repo-p", "w-p/3", "w-p/2", "w-p/5"].map(String::from).to_vec();
    let anchors = vec![("main".to_string(), "home".to_string()), ("project:p".to_string(), "repo-p".to_string())];
    // project:q has no coordinator in herdr now: its task goes last with yours, before them.
    assert_eq!(order(&have, &places, &anchors), ["home", "w-p/2", "w-p/4", "mine", "repo-p", "w-p/3", "w-p/5", "w-p/1"]);
}

#[test]
fn a_prefixed_name_is_cut_at_its_end_never_before_the_number() {
    let name = format!("PRJ-12 {}", "word ".repeat(30));
    let l = labelled("│  └─ ", &name);
    assert!(l.starts_with("│  └─ PRJ-12 word") && l.ends_with('…') && l.chars().count() == sw::flow::sidebar_entry::CAP, "{l}");
}

fn sync() { sync_sidebar(&sw::flow::live::board(true), None, sw::clock::now()); }
fn space(id: &str) -> String { task(id).at("link").t("workspace") }
fn label(w: &World, ws: &str) -> String { w.state().arr("workspaces").iter().find(|x| x.s("workspace_id") == ws).map(|x| x.t("label")).unwrap_or_default() }
/// herdr's agents list grouped by space: each space in its order, its panes in their tabs' order, as (row 1, row 2).
fn agents_list(w: &World) -> Vec<(String, String)> {
    let st = w.state();
    let mut out = vec![];
    for ws in st.arr("workspaces") {
        let mut panes: Vec<&Value> = st.arr("panes").iter().filter(|p| p.at("workspace_id") == ws.at("workspace_id") && p.b("display_agent")).collect();
        panes.sort_by_key(|p| p.t("tab_id"));
        out.extend(panes.into_iter().map(|p| (ws.t("label"), p.t("display_agent").split(" · ").take(2).collect::<Vec<_>>().join(" · "))));
    }
    out
}

#[test]
fn herdrs_sidebar_shows_the_coordinator_then_its_tasks_then_the_tasks_that_wait_for_them_with_each_reviewer_under_its_agent_and_your_tasks_last() {
    let w = world();
    let step = json!({ "files": { "hello.txt": "hello\n" }, "note": "Added the greeting." });
    w.script(json!({ "worker": [step, step, step, step] }));
    let ids: Vec<String> = ["Plan the trip", "Book the train", "Pack the bags", "Water the plants"].iter().map(|n| w.add(n)).collect();
    for id in &ids { delegate(id); settle(id); }
    let (a, b, c, d) = (&ids[0], &ids[1], &ids[2], &ids[3]);
    // a and b are the Coordinator's (all four wait for your yes), b waits for a; c is the machine coordinator's; d is yours.
    for (id, by, after) in [(a, "main", None), (b, "main", Some(a)), (c, "machine", None)] {
        update(id, |t| { t.put("coordinator", by); if let Some(x) = after { t.put("after", json!([x])); } Ok(Change::Write) }).unwrap();
    }
    // Both coordinators live in tabs of the home space, which herdr has after the tasks' spaces.
    let api = Herdr::new(Some(w.socket.clone()));
    let home = api.workspace_create(&w.root.display().to_string(), "~", &[]).unwrap();
    let (home_ws, home_pane) = (home.at("workspace").t("workspace_id"), home.at("root_pane").t("pane_id"));
    for co in [Co::top(&w.socket), Co::machine(&w.socket)] {
        sw::coordinator::state::change(co, |s| { s.put("pane", home_pane.as_str()); s.put("space", home_ws.as_str()); true }).unwrap();
    }
    w.patch(|st| { for p in st["panes"].as_array_mut().unwrap() { if p.s("pane_id") == home_pane { p["display_agent"] = json!("Coordinator · idle"); } } });
    // a's work is under review: its reviewer has a tab of its own in a's space.
    let rv = format!("{}:p90", space(a));
    w.patch(|st| {
        let mine = st["panes"].as_array().unwrap().iter().find(|p| p.s("workspace_id") == space(a)).unwrap().clone();
        let mut r = mine.clone();
        r["pane_id"] = json!(rv); r["tab_id"] = json!(format!("{}:t9", space(a))); r["terminal_id"] = json!("term_rv");
        st["panes"].as_array_mut().unwrap().push(r);
    });
    update(a, |t| { t.put("link", { let mut l = t.o("link").cloned().unwrap_or_default(); l.put("reviewer", json!({ "pane": rv })); Value::Object(l) }); Ok(Change::Write) }).unwrap();
    sync();
    let key = |id: &str| sw::flow::sidebar::space_label(&task(id));
    let list = agents_list(&w);
    let rows: Vec<(String, String)> = list.iter().filter(|(l, _)| l == "~" || ids.iter().any(|id| l.ends_with(&key(id)))).cloned().collect();
    assert_eq!(rows, vec![
        ("~".to_string(), "Coordinator · idle".to_string()),
        (format!("└─ {}", key(a)), "ready to approve · R#1".into()),
        (format!("└─ {}", key(a)), "reviewer · ready to approve".into()),
        (format!("   └─ {}", key(b)), "ready to approve · R#1".into()),
        (format!("└─ {}", key(c)), "ready to approve · R#1".into()),
        (key(d), "ready to approve · R#1".into()),
    ], "{list:#?}");
    // Put once per change of the tree: a space you move stays where you put it.
    let moves = w.state().arr("moves").len();
    assert_eq!(moves, 1);
    w.patch(|st| { let ws = st["workspaces"].as_array_mut().unwrap(); let x = ws.remove(0); ws.push(x); });
    sync();
    assert_eq!(w.state().arr("moves").len(), moves, "nothing moved again");
    // What a task says at once keeps its place in the tree.
    tell_sidebar(&task(b));
    assert_eq!(label(&w, &space(b)), format!("   └─ {}", key(b)));
    // b no longer waits for a: it stands beside it, and the spaces are put again.
    update(b, |t| { t.del("after"); Ok(Change::Write) }).unwrap();
    sync();
    let both: Vec<String> = w.state().arr("workspaces").iter().filter(|x| [space(a), space(b)].contains(&x.t("workspace_id"))).map(|x| x.t("label")).collect();
    assert!(both.len() == 2 && both[0].starts_with("├─ PRJ-") && both[1].starts_with("└─ PRJ-"), "{both:?}");
    assert_eq!(w.state().arr("moves").len(), moves + 1);
}
