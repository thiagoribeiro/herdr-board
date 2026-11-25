//! The diff key: a task's change in your diff tool (`board.diffTool`), in a herdr pane split off the board's (in the
//! popup: off the pane you came from, and the popup closes), in the task's folder, with `exit` after it so the pane
//! closes when the tool ends; against where it left main, and after an approval that lapsed only what the task itself
//! changed since that yes, with main merged in after it left out. A new yes ends that.
use crate::common;
use common::board::{open, open_with};
use common::*;
use serde_json::json;
use std::path::Path;
use sw::board::job_diff::command;
use sw::json::*;
use sw::store::settings::{change::set_value, lookup::Scope};

fn commit(dir: &Path, name: &str) -> String {
    std::fs::write(dir.join(name), "x\n").unwrap();
    git(dir, &["add", "."]); git(dir, &["commit", "-qm", name]);
    git(dir, &["rev-parse", "HEAD"]).trim().to_string()
}
fn put(id: &str, k: &str, v: serde_json::Value) { sw::store::tasks::patch(id, |t| { t.put(k, v); }).unwrap(); }
fn tool(v: &str) { set_value("board.diffTool", &json!(v), &Scope::you()).unwrap(); }
fn pane(w: &World) { w.patch(|st| st["panes"] = json!([{ "pane_id": "w1:p1", "workspace_id": "w1", "tab_id": "w1:t1", "cwd": "/", "agent": null }])); }

#[test]
fn the_key_runs_your_diff_tool_in_a_pane_beside_the_board_against_where_the_task_left_main() {
    let w = world();
    let id = w.add("Fix the page");
    let start = git(&w.root, &["rev-parse", "HEAD"]).trim().to_string();
    commit(&w.root, "a.txt");
    put(&id, "worktree", json!(w.root_str())); put(&id, "baseSha", json!(start));
    pane(&w);
    let mut b = open(&w.root, 40);
    b.s.ctx.pane = "w1:p1".into(); b.s.sel = id.clone(); b.load();
    b.press(&["alt+g"]);
    let c = calls(&w);
    assert!(c.contains(&format!("pane split w1:p1 --direction right --cwd {} --no-focus", w.root_str())), "{c:#?}");
    let new = w.state().arr("panes").last().unwrap().t("pane_id");
    // Built in: git's diff in a less that waits for your q also on a short diff, whatever LESS git would give it.
    assert!(c.contains(&format!("pane run {new} git diff --color {start} | less -+F -R; exit")), "{c:#?}");
    assert!(c.contains(&format!("pane layout --pane {new}")), "the keyboard is put on it: {c:#?}");
    assert!(!b.closed);
    // Your own tool, with or without the commit.
    tool("git diff {from} | delta --paging always");
    assert_eq!(command(&task(&id)).unwrap(), format!("git diff {start} | delta --paging always"));
    tool("lazygit");
    assert_eq!(command(&task(&id)).unwrap(), "lazygit");
    // In the popup: split off the pane you came from, and the popup closes.
    let mut p = open_with(&w.root, 40, true);
    p.s.ctx.from = "w1:p1".into(); p.s.sel = id.clone(); p.load();
    p.press(&["alt+g"]);
    assert_eq!(calls(&w).iter().filter(|c| c.starts_with("pane split w1:p1 ")).count(), 2);
    assert!(p.closed, "the popup closes on the pane");
}

#[test]
fn after_an_approval_that_lapsed_it_shows_only_the_tasks_own_change_since_your_yes_without_main_merged_in() {
    let w = world_with(Opts { repo: true, repo_settings: Some(json!({ "check.command": "true" })), ..Default::default() });
    let pass = json!({ "verdict": { "meets": true, "text": "Good.", "findings": [] } });
    w.script(json!({ "reviewer": [pass.clone(), pass] }));
    let id = to_approval(&w, "Work");
    let t = task(&id);
    let (approved, tree) = (t.t("reviewedHead"), t.t("worktree"));
    commit(&w.root, "from_main.txt"); // main moves: your yes merges it into the branch
    commit(Path::new(&tree), "late.txt"); // and the agent commits after the review you looked at
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!((t.s("stage"), t.s("lapsedApproval")), ("approval", approved.as_str()), "{}", t.s("msg"));
    tool("{from}");
    let from = command(&t).unwrap();
    assert_eq!(git(Path::new(&tree), &["diff", "--name-only", &from]).trim(), "late.txt", "main's file is not the task's change");
    assert!(git(Path::new(&tree), &["diff", "--name-only", &approved]).contains("from_main.txt"), "the approved commit alone would show main's");
    // Your new yes ends it: the diff is against main again.
    let t = approve_again(&id, "late.txt");
    assert!(!t.b("lapsedApproval"), "{t:?}");
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
