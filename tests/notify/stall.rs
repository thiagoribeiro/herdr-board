//! The stall detector's fixed rules (stall.rs) and what the board undoes by itself: too long in one stage, a pull
//! request in conflict with main (sent back to the agent under `merge.fixConflicts`, or held for you), main out of
//! reach with the time of the last try, and an agent whose program exited mid-turn (a real error, retried by itself
//! when it passes), and one merged outside the board (done by itself). Each is said on the row and in herdr's sidebar, and the first two are `board.notify` events.
use crate::common;
use crate::pull_requests_checks::{none, pr_world};
use common::*;
use serde_json::json;
use sw::housekeeping::poll_prs;
use sw::json::*;
use sw::notify::event_of;
use sw::store::settings::{change::set_value, lookup::Scope};
use sw::store::tasks::patch;

fn says(id: &str) -> String { sw::shown::row_says::sentence(&sw::shown::states::as_row(&row(id))).text() }
fn sidebar(id: &str) -> String { sw::flow::sidebar_entry::entry_of(&row(id), "", sw::clock::now()).stands }
fn conflicting(head: &Value) -> Value { let mut v = none(head.as_str().unwrap_or("")); v["mergeable"] = json!("CONFLICTING"); v }

#[test]
fn a_task_with_no_change_past_stall_after_is_stalled_on_its_row_its_sidebar_and_for_notify() {
    let w = world();
    let id = to_approval(&w, "Stuck");
    patch(&id, |t| { t.put("stage", "working"); }).unwrap();
    sw::clock::advance(44 * 60000);
    assert!(row(&id).at("stall").is_null(), "44m is not past the built-in 45m");
    sw::clock::advance(2 * 60000);
    let r = row(&id);
    assert_eq!(r.at("stall").s("event"), "stalled");
    assert!(says(&id).starts_with("No change for 46m"), "{}", says(&id));
    assert!(sidebar(&id).contains("No change for 46m"), "{}", sidebar(&id));
    assert_eq!(event_of(&task(&id)), "stalled");
    // its agent seen at work is a change: not stalled
    patch(&id, |t| { t.put("workedAt", sw::clock::now()); }).unwrap();
    assert!(row(&id).at("stall").is_null());
    // `none`: never
    sw::clock::advance(5 * 3600000);
    set_value("stall.after", &json!("none"), &Scope::you()).unwrap();
    assert!(row(&id).at("stall").is_null());
}

#[test]
fn a_pull_request_in_conflict_goes_back_to_its_agent_by_itself() {
    let w = pr_world(json!({}));
    let id = to_approval(&w, "Conflicted");
    w.script(json!({ "worker": [{ "note": "Merged main in." }] }));
    w.gh_says(json!({ "view": conflicting(task(&id).at("prHead")) }));
    poll_prs();
    let t = task(&id);
    assert!(t.b("prConflict"));
    let last = prompts(&w, t.at("link").s("pane")).last().cloned().unwrap_or_default();
    assert!(last.contains("conflicts with main") && last.contains("git merge origin/main"), "{last}");
    assert_eq!(t.i("prRounds"), 1);
    // GitHub says it merges cleanly again: the conflict is over
    let mut ok = none(task(&id).s("prHead")); ok["mergeable"] = json!("MERGEABLE");
    w.gh_says(json!({ "view": ok }));
    poll_prs();
    assert!(!task(&id).b("prConflict"));
}

#[test]
fn with_fix_conflicts_off_a_conflict_waits_for_you_with_the_reason_and_is_a_notify_event() {
    let w = pr_world(json!({ "merge.fixConflicts": false }));
    let id = to_approval(&w, "Held");
    let before = prompts(&w, task(&id).at("link").s("pane")).len();
    w.gh_says(json!({ "view": conflicting(task(&id).at("prHead")) }));
    poll_prs();
    let t = task(&id);
    assert_eq!(t.s("stage"), "approval");
    assert_eq!(prompts(&w, t.at("link").s("pane")).len(), before, "nothing sent");
    assert_eq!(t.at("prFailure").s("title"), "Pull request conflicts with main");
    assert!(says(&id).starts_with("Pull request conflicts with main"), "{}", says(&id));
    assert_eq!(event_of(&t), "conflict");
    assert_eq!(sw::shown::turn::whose(&sw::shown::states::as_row(&row(&id))), "you");
}

#[test]
fn main_out_of_reach_says_when_it_was_last_tried_and_is_the_boards_turn() {
    let w = world();
    let id = to_approval(&w, "Offline");
    let at = sw::clock::now();
    patch(&id, |t| { t.put("syncBlocked", json!({ "why": "main could not be fetched (Could not resolve host: github.com)", "at": at })); }).unwrap();
    let r = row(&id);
    assert_eq!(r.at("stall").s("event"), "unreachable");
    let hm = chrono::DateTime::from_timestamp_millis(at).unwrap().with_timezone(&chrono::Local).format("%H:%M").to_string();
    assert!(says(&id).starts_with(&format!("main could not be fetched, last try {hm}")), "{}", says(&id));
    assert_eq!(sw::shown::turn::whose(&sw::shown::states::as_row(&r)), "board");
    assert_eq!(event_of(&task(&id)), "needs you", "not an event of its own: the board keeps trying");
}

#[test]
fn an_agent_whose_program_exited_on_a_connection_error_is_retried_by_itself() {
    let w = cut_world("60000", json!({}));
    // it takes the task and works, then its program exits in the middle of the turn
    w.script(json!({ "worker": [{ "status": "working", "then": { "screen": "⏺ Reading files\n  API Error: Connection lost mid-response. The response above may be incomplete.\n", "exit": true } }] }));
    let id = w.add("Dropped");
    delegate(&id);
    assert_eq!(settle(&id).s("stage"), "working");
    w.go_on(&task(&id));
    let t = settle(&id);
    assert_eq!(t.s("stage"), "failed");
    let wait = sw::store::retries::waiting_of(&t).cloned().unwrap_or_default();
    assert_eq!((wait.s("who"), wait.s("reason")), ("agent", "connection dropped"), "{t:?}");
    assert!(t.at("failure").s("detail").contains("Connection lost mid-response"));
    assert!(says(&id).starts_with("Agent's connection dropped, trying again"), "{}", says(&id));
}

#[test]
fn an_agent_whose_program_exited_for_no_known_reason_is_a_real_failure_that_retry_sends() {
    let w = cut_world("60000", json!({}));
    // it takes the task and works, then its program exits in the middle of the turn
    w.script(json!({ "worker": [{ "status": "working", "then": { "screen": "Segmentation fault\n", "exit": true } }] }));
    let id = w.add("Gone");
    delegate(&id);
    assert_eq!(settle(&id).s("stage"), "working");
    w.go_on(&task(&id));
    let t = settle(&id);
    assert_eq!(t.s("stage"), "failed");
    assert!(sw::store::retries::waiting_of(&t).is_none());
    assert!(t.s("msg").starts_with("Agent stopped: The agent's program exited."), "{}", t.s("msg"));
    let said = sw::actions::finish_line::send_error(&id).map(|s| s.msg).unwrap_or_else(|e| e.msg);
    assert!(!said.contains("no agent error to send"), "{said}");
}

/// Merged outside the board (on GitHub, by you) while the task works: done, said so, and notified (TRM-4).
#[test]
fn a_pull_request_merged_on_github_while_the_task_works_makes_it_done() {
    let w = pr_world(json!({}));
    let id = to_approval(&w, "Merged elsewhere");
    // the human typed in the agent's terminal: the task is at work again, and then merges its pull request on GitHub
    patch(&id, |t| { t.put("stage", "working"); }).unwrap();
    w.gh_says(json!({ "view": { "state": "MERGED", "headRefOid": task(&id).at("prHead") } }));
    poll_prs();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done");
    assert!(t.b("mergedOutside"));
    assert_eq!(sw::notify::event_of(&t), "merged outside");
}
