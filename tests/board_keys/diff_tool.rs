//! The diff key: a task's change in your diff tool (`review.tool`), in a herdr pane split off the board's, in the task's
//! folder, with `exit` after it so the pane closes when the tool ends; against where it left main, and since your
//! approval once the branch moved on from the commit you approved.
use crate::common;
use common::board::open;
use common::*;
use serde_json::json;
use sw::board::update::diff::command;
use sw::json::*;

fn commit(w: &World, name: &str) -> String {
    std::fs::write(w.root.join(name), "x\n").unwrap();
    git(&w.root, &["add", "."]); git(&w.root, &["commit", "-qm", name]);
    git(&w.root, &["rev-parse", "HEAD"]).trim().to_string()
}
fn put(id: &str, k: &str, v: serde_json::Value) { sw::store::tasks::patch(id, |t| { t.put(k, v); }).unwrap(); }

#[test]
fn the_key_runs_your_diff_tool_beside_the_board_against_main_then_since_your_approval() {
    let w = world();
    let id = w.add("Fix the page");
    let start = git(&w.root, &["rev-parse", "HEAD"]).trim().to_string();
    let done = commit(&w, "a.txt");
    put(&id, "worktree", json!(w.root_str())); put(&id, "baseSha", json!(start));
    w.patch(|st| st["panes"] = json!([{ "pane_id": "w1:p1", "workspace_id": "w1", "tab_id": "w1:t1", "cwd": "/", "agent": null }]));
    let mut b = open(&w.root, 40);
    b.s.ctx.pane = "w1:p1".into(); b.s.sel = id.clone(); b.load();
    b.press(&["alt+g"]);
    let c = calls(&w);
    assert!(c.contains(&format!("pane split w1:p1 --direction right --cwd {} --no-focus", w.root_str())), "{c:#?}");
    let new = w.state().arr("panes").last().unwrap().t("pane_id");
    assert!(c.contains(&format!("pane run {new} git diff {start}; exit")), "{c:#?}");
    assert!(c.contains(&format!("pane layout --pane {new}")), "the keyboard is put on it: {c:#?}");

    // Approved at this commit: nothing moved since, so still against main. A commit after it: only what is new.
    put(&id, "approvedHead", json!(done));
    assert_eq!(command(&task(&id)).unwrap(), format!("git diff {start}"));
    commit(&w, "b.txt");
    assert_eq!(command(&task(&id)).unwrap(), format!("git diff {done}"));
    // Your own tool, with or without the commit.
    sw::store::settings::change::set_value("review.tool", &json!("git diff {from} | delta"), &sw::store::settings::lookup::Scope::you()).unwrap();
    assert_eq!(command(&task(&id)).unwrap(), format!("git diff {done} | delta"));
    sw::store::settings::change::set_value("review.tool", &json!("lazygit"), &sw::store::settings::lookup::Scope::you()).unwrap();
    assert_eq!(command(&task(&id)).unwrap(), "lazygit");
}

#[test]
fn outside_herdr_and_on_the_web_board_the_key_only_says_so() {
    let w = world();
    let id = w.add("Fix the page");
    let mut b = open(&w.root, 40);
    b.s.sel = id; b.load();
    b.press(&["alt+g"]);
    assert!(b.s.msg.contains("opens in a herdr pane beside the board"), "{}", b.s.msg);
    assert!(!calls(&w).iter().any(|c| c.starts_with("pane split")));
}
