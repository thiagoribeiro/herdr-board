//! The board opened at herdr's start (ensure-open): one board, with no tasks too; the one that runs or that herdr
//! brought back is kept and never closed; a second call never opens a second board; a note a crashed board left is no
//! board; the keyboard goes only to a board it opened itself, and only when asked.
use crate::common;
use common::*;
use serde_json::json;
use sw::board::run::ensure::{ensure, PANE_LABEL};
use sw::json::*;

fn boards(w: &World) -> Vec<String> { w.state().arr("panes").iter().filter(|p| p.s("label") == PANE_LABEL).map(|p| p.t("pane_id")).collect() }
fn focus(w: &World) -> Vec<String> { w.state().arr("focus").iter().map(text).collect() }
/// herdr with one space and nothing of the board's in it.
fn herdr(w: &World) {
    let root = w.root_str();
    w.patch(|s| { s["workspaces"] = json!([{ "workspace_id": "w1", "label": "home", "cwd": root }]); s["panes"] = json!([{ "pane_id": "w1:p1", "workspace_id": "w1", "tab_id": "w1:t1", "terminal_id": "term_1", "cwd": root, "agent": null, "focused": true }]); s["tabs"] = json!([{ "tab_id": "w1:t1", "workspace_id": "w1", "label": "1" }]); });
}
fn note(pane: &str, pid: i64) { sw::store::files::write_json(&sw::store::files::data_dir().join("board-pane-default.json"), &json!({ "pane": pane, "pid": pid })).unwrap(); }

#[test]
fn with_no_tasks_herdrs_start_opens_one_board_in_a_tab_of_its_own_and_gives_it_the_keyboard_when_asked() {
    let w = world();
    herdr(&w);
    assert!(sw::store::tasks::list(Default::default()).is_empty(), "no tasks at all");
    assert_eq!(sw::store::settings::lookup::setting("board.openOnStartup", None), json!(true));
    assert_eq!(sw::store::settings::lookup::setting("board.focusOnStartup", None), json!(true));
    let pane = ensure(&w.socket, true);
    assert_eq!(boards(&w), vec![pane.clone()]);
    let tab = w.state().arr("panes").iter().find(|p| p.s("pane_id") == pane).unwrap().t("tab_id");
    assert_eq!(w.state().arr("tabs").iter().find(|t| t.s("tab_id") == tab).unwrap().s("label"), "board");
    assert!(focus(&w).contains(&format!("tab {tab}")), "the keyboard is on it: {:?}", focus(&w));
    assert!(!calls(&w).iter().any(|c| c.starts_with("agent")), "herdr only: no agent asked anything");
}

#[test]
fn a_board_that_is_there_already_is_kept_where_it_is_and_never_closed_nor_opened_twice() {
    let w = world();
    herdr(&w);
    // Two calls, the second while the first one's board has not yet said where it runs (as two starts racing, one
    // after the other under the lock): one board, and the second moves nothing.
    let first = ensure(&w.socket, true);
    let seen = focus(&w).len();
    assert_eq!(ensure(&w.socket, true), first);
    assert_eq!(boards(&w), vec![first.clone()]);
    assert_eq!(focus(&w).len(), seen, "the keyboard stays where it is");
    // A board that runs and has said where (its note, its process alive): that one, whatever its pane is called.
    w.patch(|s| s["panes"].as_array_mut().unwrap().retain(|p| p.s("label") != PANE_LABEL));
    note("w1:p1", std::process::id() as i64);
    assert_eq!(ensure(&w.socket, true), "w1:p1");
    assert!(boards(&w).is_empty(), "no second board beside the one that runs");
    assert_eq!(focus(&w).len(), seen);
    assert!(!calls(&w).iter().any(|c| c.contains("popup") || c.contains("close")), "nothing closed: {:?}", calls(&w));
}

#[test]
fn after_a_restart_or_a_crash_a_board_that_runs_is_kept_and_the_bare_shell_herdr_brought_back_gives_way_to_a_new_board() {
    let w = world();
    herdr(&w);
    // The crashed board's note names a pane that is gone, and its process is dead: no board.
    note("w1:p9", 999_999_999);
    // A board pane where the board runs (it has not said so yet): that one, not a second.
    let root = w.root_str();
    let pane = |id: &str, tab: &str, board: bool, focused: bool| json!({ "pane_id": id, "workspace_id": "w1", "tab_id": tab, "terminal_id": format!("term_{id}"), "cwd": root, "agent": null, "label": PANE_LABEL, "board": board, "focused": focused });
    w.patch(|s| s["panes"].as_array_mut().unwrap().push(pane("w1:p7", "w1:t7", true, false)));
    assert_eq!(ensure(&w.socket, false), "w1:p7");
    assert_eq!(boards(&w), vec!["w1:p7".to_string()]);
    // herdr brought the board's tab back after a restart as a bare shell, with the keyboard on it (as herdr 0.9.5 does):
    // a new board opens, takes the keyboard there, and the idle shell is closed.
    w.patch(|s| { s["panes"].as_array_mut().unwrap().retain(|p| p.s("label") != PANE_LABEL); s["panes"].as_array_mut().unwrap().push(pane("w1:p8", "w1:t8", false, true)); s["tabs"].as_array_mut().unwrap().push(json!({ "tab_id": "w1:t8", "workspace_id": "w1", "label": "board" })); });
    let new = ensure(&w.socket, false);
    assert!(!new.is_empty() && new != "w1:p8" && new != "w1:p9");
    assert_eq!(boards(&w), vec![new.clone()], "one board, and the shell gone");
    let tab = w.state().arr("panes").iter().find(|p| p.s("pane_id") == new).unwrap().t("tab_id");
    assert!(focus(&w).contains(&format!("tab {tab}")), "the keyboard where it was: {:?}", focus(&w));
    // A shell where something runs is not closed.
    w.patch(|s| { s["panes"].as_array_mut().unwrap().retain(|p| p.s("label") != PANE_LABEL); let mut b = pane("w1:p6", "w1:t6", false, false); b["busy"] = json!(true); s["panes"].as_array_mut().unwrap().push(b); });
    let again = ensure(&w.socket, false);
    assert_eq!(boards(&w).len(), 2, "the busy one stays beside the new board"); assert!(boards(&w).contains(&again));
}

#[test]
fn with_focus_off_the_board_opens_behind() {
    let w = world();
    herdr(&w);
    let seen = focus(&w).len();
    let pane = ensure(&w.socket, false);
    assert_eq!(boards(&w), vec![pane]);
    assert_eq!(focus(&w)[seen..].to_vec(), vec!["board", "workspace w1", "tab w1:t1"], "herdr gave the new tab the keyboard: back where it was");
}

#[test]
fn with_the_popup_chosen_herdrs_start_opens_nothing_the_popup_is_the_keys_only() {
    let w = world_with(Opts { repo: true, settings: json!({ "board.open": "popup" }), ..Default::default() });
    herdr(&w);
    assert_eq!(ensure(&w.socket, true), "");
    assert!(boards(&w).is_empty());
    assert!(w.state().at("boardPlacement").is_null(), "no popup either");
}
