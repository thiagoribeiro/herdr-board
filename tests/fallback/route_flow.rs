//! A routed task in its flow: its entry's program, model and effort start it; its work failing review hands it to the
//! next entry of its chain after `route.escalateAfter` rounds (not `review.rounds`), in the same worktree with the
//! open findings and that entry's own effort; `details` says so, and a new delegation starts on the first entry again.
use crate::common;
use common::*;
use serde_json::json;
use sw::json::*;
use sw::store::handovers::{handover_line, handovers};
use sw::store::settings::lookup::{lookup, Scope};
use sw::store::settings::places::yours;

const FAIL: &str = r#"{ "verdict": { "meets": false, "findings": [{ "severity": "major", "subject": "The file says v1, the task asked for v2", "where": "f.txt:1", "proof": "cat f.txt" }], "text": "Not what was asked." } }"#;
fn launches(w: &World) -> Vec<String> { calls(w).into_iter().filter(|c| c.starts_with("agent start") && c.contains("--kind claude")).collect() }

#[test]
fn a_task_routed_to_leve_whose_review_fails_escalates_to_padrao_in_the_same_worktree_after_one_round() {
    let w = cut_world("30", json!({ "review.rounds": 3 }));
    let fail: serde_json::Value = serde_json::from_str(FAIL).unwrap();
    w.script(json!({ "worker": [step(1), { "status": "working" }], "reviewer": [fail] }));
    let id = w.add("Typo in the readme");
    sw::store::route::choose(&id, "leve", "typo, 1 arquivo", "coordinator").unwrap();
    delegate(&id);
    let mut t = settle(&id);
    for _ in 0..3 { if t.s("stage") != "working" || handovers(&t).is_empty() { t = settle(&id); } }
    assert_eq!(t.s("stage"), "working", "{}", t.at("failure"));
    let l = launches(&w);
    assert_eq!(l.len(), 2, "{l:?}");
    assert!(l[0].contains("--model haiku") && l[0].contains("--effort low"), "{}", l[0]);
    assert!(l[1].contains("--model sonnet") && l[1].contains("--effort medium"), "{}", l[1]);
    assert_eq!(calls(&w).iter().filter(|c| c.starts_with("worktree create")).count(), 1, "the same worktree");
    assert_eq!(handover_line(&handovers(&t)[0]), "claude/sonnet took over from claude/haiku: its work failed review (1 round on leve, route.escalateAfter)");
    assert_eq!((t.at("route").s("entry"), t.at("route").arr("escalated").len(), t.s("on")), ("padrao", 1, "claude/sonnet"));
    let told = prompts(&w, t.at("link").s("pane"))[0].clone();
    assert!(told.contains("The last review of this work failed (Fail · 1 major), and what it found is still open"), "{told}");
    let f = lookup("worker.effort", &Scope::task(&t), &yours()).unwrap();
    assert_eq!((f.value, f.from), (json!("medium"), "routed: padrao, escalated from leve — \"typo, 1 arquivo\"".to_string()));
    assert_eq!(lookup("worker.fallback", &Scope::task(&t), &yours()).unwrap().value, json!(["claude/opus"]));
    let d = sw::actions::finish_line::details(&id).unwrap();
    assert_eq!(d.arr("handovers").iter().map(|h| (h.t("from"), h.t("to"))).collect::<Vec<_>>(), vec![("claude/haiku".to_string(), "claude/sonnet".to_string())]);
    assert_eq!(d.at("route").s("words"), "routed: padrao, escalated from leve — \"typo, 1 arquivo\"");
    // Its tokens are counted, for `models stats`, though no limit is said.
    assert!(sw::limits::run::limit_said(&t));
}

#[test]
fn escalate_after_two_sends_the_work_back_once_and_the_last_entry_has_the_usual_rounds() {
    let w = cut_world("30", json!({ "review.rounds": 1, "route.escalateAfter": 2 }));
    std::fs::write(sw::agents::models::catalog::path(), "[[entry]]\nname = \"leve\"\nagent = \"claude/haiku\"\nescalate_to = \"forte\"\n[[entry]]\nname = \"forte\"\nagent = \"claude/opus\"\n").unwrap();
    let fail: serde_json::Value = serde_json::from_str(FAIL).unwrap();
    let mut again = fail.clone(); again["verdict"]["findings"][0]["id"] = json!("F1");
    w.script(json!({ "worker": [step(1), { "note": "Tried." }, { "note": "Tried more." }, { "status": "working" }], "reviewer": [fail.clone(), again.clone(), fail, again] }));
    let id = w.add("Feature");
    sw::store::route::choose(&id, "leve", "", "coordinator").unwrap();
    delegate(&id);
    let mut t = settle(&id);
    for _ in 0..6 { if t.s("stage") != "working" && t.s("stage") != "failed" && t.s("stage") != "approval" { t = settle(&id); } }
    let lines: Vec<String> = handovers(&t).iter().map(handover_line).collect();
    assert_eq!(lines, vec!["claude/opus took over from claude/haiku: its work failed review (2 rounds on leve, route.escalateAfter)"], "{}", t.at("failure"));
    // A new delegation starts on the first entry again.
    let _ = sw::actions::talk::stop(&id);
    let mut cur = task(&id);
    sw::store::route::at_delegate(&mut cur);
    assert_eq!(cur.at("route").s("entry"), "leve");
}
