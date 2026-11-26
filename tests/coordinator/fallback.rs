//! Coordinator outages preserve original requests, queued steering, memory, permissions and history without retry loops or budget bypass.
use crate::common;
use common::*;
use sw::{coordinator::{co::Co, fallback, pump, requests, state}, json::*};
fn world_with_fallback(list: Value) -> World {
    let w = world_with(Opts { repo: true, settings: json!({ "coordinator.agent": "claude/sonnet", "coordinator.access": "auto", "coordinator.fallback": list }), ..Default::default() });
    let home = w.base.join("coordinator"); std::fs::create_dir_all(&home).unwrap();
    unsafe { std::env::set_var("HERDR_BOARD_COORDINATOR_DIR", home); }
    crate::agent_models::equivalents::pairs(); w
}
fn drain() { flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); }); }
fn start_kinds(w: &World) -> Vec<String> {
    calls(w).into_iter().filter(|c| c.starts_with("agent start")).map(|c| c.split("--kind ").nth(1).unwrap().split_whitespace().next().unwrap().into()).collect()
}
#[test]
fn an_unavailable_coordinator_continues_on_its_peer_with_original_identity_memory_and_access() {
    for ending in ["claude-usage-limit-made", "claude-model-not-found", "claude-overloaded-made"] {
        let w = world_with_fallback(json!(["codex/gpt-6.1-sol"])); let co = Co::top(&w.socket);
        let task = w.add("Existing work");
        sw::coordinator::context::remember(&co, &json!({ "decisions": ["Keep existing work"], "constraints": ["No unasked tasks"], "questions": [] })).unwrap();
        w.script(json!({ "coordinator": [{ "ends": ending }, { "status": "working" }] }));
        requests::send(&w.root, "Inspect the existing task only", false, &task, &co).unwrap(); drain();
        let c = state::get(&co).unwrap();
        assert_eq!(start_kinds(&w), vec!["claude", "codex"], "{ending}: {}", c.at("error"));
        assert_eq!((c.at("run").s("kind"), c.at("run").s("effort"), c.at("run").s("access")), ("codex", "medium", "auto"));
        let previous = &c.arr("checkpoints")[0];
        assert_eq!(previous.s("reason"), "availability"); assert!(previous.b("session") && previous.b("transcript"));
        assert_eq!(c.s("typedId"), previous.arr("requests")[0].s("id"));
        assert_eq!(c.at("memory").at("content").arr("decisions"), &[json!("Keep existing work")]);
        let words = prompts(&w, c.s("pane")).join("\n");
        assert!(words.contains("never blindly repeat") && words.contains("Inspect the existing task only") && words.contains(&task), "{words}");
        assert_eq!(sw::store::tasks::list(Default::default()).len(), 1);
        assert!(calls(&w).iter().any(|c| c.starts_with("agent start") && c.contains("--kind codex") && c.contains("--approve-for-me")));
    }
}
#[test]
fn exhausted_peers_stop_once_and_new_conversations_can_use_the_primary_again() {
    let w = world_with_fallback(json!(["codex/gpt-6.1-sol", "claude/sonnet"])); let co = Co::top(&w.socket);
    w.script(json!({ "coordinator": [{ "ends": "claude-usage-limit-made" }, { "ends": "codex-unauthorized" }] }));
    requests::send(&w.root, "Inspect only", false, "", &co).unwrap(); drain();
    w.patch(|st| { for p in st["panes"].as_array_mut().unwrap() { p["agent"] = Value::Null; } });
    sw::clock::set(Some(sw::clock::now() + 5000)); pump::tick(&co); drain();
    let c = state::get(&co).unwrap();
    assert_eq!(start_kinds(&w), vec!["claude", "codex"]); assert!(c.b("fallbackExhausted"), "{c:?}");
    for _ in 0..3 { pump::tick(&co); drain(); }
    assert_eq!(start_kinds(&w).len(), 2);
    requests::fresh(None, &co).unwrap(); drain();
    assert_eq!(state::get(&co).at("run").s("kind"), "claude");
    assert!(!state::get(&co).b("fallbackExhausted"));
}
#[test]
fn a_persisted_transition_keeps_queued_steering_and_a_task_senders_restrictions() {
    let w = world_with_fallback(json!(["codex/gpt-6.1-sol"])); let co = Co::top(&w.socket);
    let sender = json!({ "task": w.add("Sender"), "role": "agent" });
    let queued = json!({ "id": "next", "text": "Then inspect status", "said": "Then inspect status", "at": sw::clock::now(), "where": {} });
    state::change(&co, |c| {
        c.put("run", json!({ "kind": "claude", "model": "sonnet", "access": "auto" })); c.put("session", "before");
        c.put("last", "Ask about my task"); c.put("typedId", "original"); c.put("typedFrom", sender.clone());
        c.put("where", json!([{ "id": "original", "said": "Ask about my task", "sentAt": 1 }]));
        c.put("queue", json!([queued])); true
    }).unwrap();
    assert!(fallback::prepare(&co, "Usage limit reached").unwrap());
    assert_eq!(state::get(&co).at("want").s("what"), "handover");
    w.script(json!({ "coordinator": [{ "status": "working" }, { "status": "working" }] }));
    pump::pump(&co); drain();
    let c = state::get(&co).unwrap();
    let text = prompts(&w, c.s("pane")).join("\n");
    assert!(text.contains("Ask about my task") && text.contains("Then inspect status"), "{text}");
    let recovery = c.arr("where").iter().find(|r| r.s("id") == "original").unwrap();
    assert_eq!(recovery.at("request").at("sender"), &sender);
}
#[test]
fn explicit_none_content_refusals_stops_and_shared_budget_stops_never_switch_programs() {
    let w = world_with_fallback(json!(["codex/gpt-6.1-sol"])); let co = Co::top(&w.socket);
    for field in ["stopped", "budgetStop"] {
        state::change(&co, |c| { c.put("run", json!({ "kind": "claude", "model": "sonnet" })); c.put(field, true); true }).unwrap();
        assert!(!fallback::prepare(&co, "Usage limit reached").unwrap());
        state::change(&co, |c| { c.del(field); true }).unwrap();
    }
    assert!(!fallback::prepare(&co, "The model refused under its safety policy").unwrap());
    assert!(!fallback::prepare(&co, "context window exceeded").unwrap());
    sw::store::settings::change::set_value("coordinator.fallback", &json!("none"), &sw::store::settings::lookup::Scope::you()).unwrap();
    assert!(!fallback::prepare(&co, "Usage limit reached").unwrap());
    assert!(start_kinds(&w).is_empty());
}
