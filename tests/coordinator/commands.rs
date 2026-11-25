//! Commands typed in the board's chat (`!status`, `!start`, `!stop`, `!approve`, `!tell`) run at once, through the
//! command line's own actions and guards, and are answered in the Coordinator's chat with no Coordinator turn; any
//! other words go to the Coordinator unchanged.
use super::coord_world;
use crate::common;
use common::board::open;
use common::*;
use serde_json::json;
use sw::coordinator::requests::send;
use sw::json::*;

/// Typed in the chat: the board's answer to it, and the words that went to the Coordinator.
fn say(w: &World, text: &str) -> (String, Vec<String>) {
    let c = send(&w.root, text, false, "", w.socket.as_str()).unwrap_or_else(|e| panic!("{text}: {}", e.msg));
    let answer = c.arr("commands").last().filter(|m| m.s("said") == text).map(|m| m.t("reply")).unwrap_or_default();
    (answer, c.arr("queue").iter().map(|m| m.t("said")).collect())
}
fn waiting(w: &World, title: &str, meets: bool) -> String {
    let id = w.add(title);
    let review = json!({ "text": "Read.", "meets": meets, "pass": meets, "gate": "major", "findings": if meets { json!([]) } else { json!([{ "id": "F1", "severity": "major", "subject": "No test", "where": "a.rs" }]) } });
    sw::store::tasks::update(&id, |t| { t.put("stage", "approval"); t.put("app", "claude"); t.put("agent", "claude"); t.put("review", review.clone()); t.put("reviewRound", 1); t.put("summary", "Done."); Ok(sw::store::tasks::Change::Write) }).unwrap();
    id
}

#[test]
fn status_typed_on_the_board_is_answered_in_the_coordinators_chat_without_a_turn() {
    let w = coord_world();
    let id = w.add("Fix the login");
    let mut s = open(&w.root, 34); s.s.side = false;
    s.typed("!status");
    let fx = s.hold("enter");
    s.run(fx);
    let c = sw::coordinator::state::get(w.socket.as_str()).unwrap();
    assert!(c.arr("queue").is_empty() && !c.b("run"), "no Coordinator turn: {c:?}");
    let talk = sw::coordinator::look::look(w.socket.as_str()).talk;
    let said: Vec<(bool, String)> = talk.iter().map(|m| (m.you, m.text.clone())).collect();
    assert_eq!(said[said.len() - 2], (true, "!status".to_string()), "{said:?}");
    assert!(said[said.len() - 1].1.contains(&short(&id)) && said[said.len() - 1].1.contains("Fix the login"), "{said:?}");
    s.load();
    let c = crate::board_chat::chat(&mut s, 120);
    assert!(c.boxed[0].contains(&format!("{}     Fix the login", short(&id))) && c.boxed[0].contains("delegate it"), "the answer is the chat's latest message: {}", c.lines.join("\n"));
    // one task, by its key
    let (a, q) = say(&w, &format!("!status {}", short(&id)));
    assert!(a.contains("Fix the login") && q.is_empty(), "{a}");
}

#[test]
fn approve_keeps_every_guard_and_does_only_what_was_typed() {
    let w = coord_world();
    let bad = waiting(&w, "Add an undo", false);
    let good = waiting(&w, "Cap titles", true);
    let (a, q) = say(&w, &format!("!approve {}", short(&bad)));
    assert!(a.contains("was not merged: its last review failed") && q.is_empty(), "{a}");
    assert_eq!(task(&bad).s("stage"), "approval", "refused, as approve refuses it");
    assert!(!a.contains("proj/"), "said with the task's key: {a}");
    let (a, _) = say(&w, &format!("!approve {} --anyway", short(&bad)));
    assert!(a.contains("was not merged"), "nothing but the id is read: {a}");
    let (a, _) = say(&w, &format!("!approve {}", short(&good)));
    assert_ne!(task(&good).s("stage"), "approval", "{a}");
    assert_eq!(task(&bad).s("stage"), "approval");
}

#[test]
fn start_tell_and_stop_run_the_command_lines_actions() {
    let w = coord_world();
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let id = w.add("Make f");
    let k = short(&id);
    let (a, _) = say(&w, &format!("!start {k}"));
    assert!(a.contains("an agent is on it"), "{a}");
    super::drain();
    let t = task(&id);
    assert!(t.at("link").b("pane"), "{t:?}");
    let (a, _) = say(&w, &format!("!tell {k} use the second file\ntoo"));
    assert!(!a.is_empty(), "{a}");
    assert!(prompts(&w, t.at("link").s("pane")).iter().any(|p| p.contains("use the second file\ntoo")), "{:?}", prompts(&w, t.at("link").s("pane")));
    let (a, q) = say(&w, &format!("!stop {k}"));
    assert!(a.contains("stopped"), "{a}");
    assert!(q.is_empty());
}

#[test]
fn other_words_go_to_the_coordinator_unchanged_and_a_command_without_its_task_says_how() {
    let w = coord_world();
    let (a, q) = say(&w, "!stop");
    assert!(a.contains("needs a task") && a.contains("!tell <id> <text>") && q.is_empty(), "{a}");
    let (a, _) = say(&w, "!tell 1");
    assert!(a.contains("needs the text"), "{a}");
    let (a, _) = say(&w, "!stop 99");
    assert!(a.contains("No task 99"), "{a}");
    for words in ["/review-pr the login", "!thanks", "status 1", "stop it, please"] { let (a, q) = say(&w, words); assert!(a.is_empty() && q.last().map(|x| x.as_str()) == Some(words), "{words}: {q:?}"); }
}
