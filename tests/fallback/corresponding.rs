//! Corresponding task workers and reviewers resume the existing work and preserve explicit permissions, effort and quality gates.
use crate::common;
use common::*;
use sw::json::*;
use crate::agent_models::equivalents::pairs;

#[test]
fn a_provider_failure_uses_its_peer_and_that_peers_effort_in_the_same_worktree() {
    for ending in ["claude-usage-limit-made", "claude-model-not-found"] {
        let w = cut_world("30", json!({ "worker.agent": "claude/sonnet", "worker.access": "auto", "fallback.corresponding": true })); pairs();
        w.script(json!({ "worker": [{ "files": { "half.txt": "half\n" }, "commit": "half", "ends": ending }, { "status": "working" }] }));
        let id = w.add("Continue existing work"); delegate(&id); let t = settle(&id);
        assert_eq!(t.s("stage"), "working", "{}", t.at("failure"));
        assert_eq!(starts(&w), vec!["claude", "codex"]);
        let h = &t.arr("handovers")[0];
        assert_eq!((h.s("reasonType"), h.s("correspondingEntry"), h.s("family")), ("availability", "routine_codex", "routine"));
        let launches = calls(&w); let last = launches.iter().find(|c| c.starts_with("agent start") && c.contains("--kind codex")).unwrap();
        assert!(last.contains("gpt-6.1-sol") && last.contains("medium") && last.contains("--approve-for-me"), "{last}");
        assert!(!last.contains("dangerously"), "a peer keeps the native access contract");
        assert_eq!(std::fs::read_to_string(std::path::Path::new(t.s("worktree")).join("half.txt")).unwrap(), "half\n");
        assert!(prompts(&w, t.at("link").s("pane")).last().unwrap().contains("taking this task over"));
        assert!(t.at("spent").b("files"), "the earlier worker's evidence survives the handover");
    }
}

#[test]
fn a_routed_outage_keeps_its_quality_tier_and_failed_review_escalates_instead_of_shopping_peers() {
    let w = cut_world("30", json!({ "fallback.corresponding": true })); pairs();
    w.script(json!({ "worker": [{ "ends": "claude-usage-limit-made" }, { "status": "working" }] }));
    let id = w.add("Routed outage"); sw::store::route::choose(&id, "routine", "", "coordinator", &[]).unwrap(); delegate(&id); let t = settle(&id);
    assert_eq!((t.s("agent"), t.at("route").s("entry"), t.at("route").arr("escalated").len()), ("codex", "routine", 0));
    assert_eq!(sw::store::corresponding::effort(&t, "worker", t.s("on")), Some("medium".into()));
    sw::actions::talk::stop(&id).unwrap();
    w.script(json!({ "worker": [{ "files": { "result.txt": "incomplete\n" }, "commit": "incomplete", "note": "Ready." }, { "status": "working" }], "reviewer": [{ "verdict": { "meets": false, "text": "Still wrong", "findings": [{ "severity": "major", "subject": "Wrong result", "where": "result.txt:1", "proof": "cat result.txt" }] } }] }));
    let next = w.add("Needs stronger reasoning"); sw::store::route::choose(&next, "routine", "", "coordinator", &[]).unwrap(); delegate(&next); let t = settle(&next);
    assert_eq!(t.s("on"), "claude/opus", "{}", t.at("failure"));
    assert!(!t.b("workerEquivalent"));
    assert_eq!(t.at("route").s("entry"), "complex");
}

#[test]
fn a_cut_off_reviewer_uses_its_own_correspondent_as_the_same_independent_review() {
    for ending in ["codex-unauthorized", "codex-model-not-supported"] {
        let w = cut_world("30", json!({ "reviewer.agent": "codex/gpt-6.1-sol", "reviewer.effort": "low", "fallback.corresponding": true })); pairs();
        w.script(json!({ "worker": [{ "files": { "done.txt": "done\n" }, "commit": "work", "note": "Done." }],
            "reviewer": [{ "ends": ending }, { "verdict": { "meets": true, "text": "Checked it", "findings": [] } }] }));
        let id = w.add("Independent review with availability fallback"); delegate(&id); let t = settle(&id);
        assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
        assert_eq!(t.s("reviewerOn"), "claude/sonnet");
        let h = t.arr("handovers").iter().find(|h| h.s("role") == "reviewer").unwrap();
        assert_eq!((h.s("correspondingEntry"), h.s("reasonType")), ("routine", "availability"));
        let launches = calls(&w);
        assert!(launches.iter().any(|c| c.starts_with("agent start") && c.contains("--kind claude") && c.contains("--model sonnet") && c.contains("--effort high")), "{launches:?}");
        let history = sw::store::reviews::review_history(&t);
        assert_eq!(history.len(), 1); assert_eq!(history[0].i("round"), 1);
    }
}

#[test]
fn a_reviewers_correspondent_preserves_the_effort_explicitly_set_on_the_task() {
    let w = cut_world("30", json!({ "reviewer.agent": "codex/gpt-6.1-sol", "fallback.corresponding": true })); pairs();
    w.script(json!({ "worker": [{ "files": { "done.txt": "done\n" }, "commit": "work", "note": "Done." }],
        "reviewer": [{ "ends": "codex-unauthorized" }, { "verdict": { "meets": true, "text": "Checked it", "findings": [] } }] }));
    let id = w.add("Review with explicit effort");
    sw::store::settings::change::set_value("reviewer.effort", &json!("medium"), &sw::store::settings::lookup::Scope::task(&task(&id))).unwrap();
    delegate(&id); let t = settle(&id);
    assert_eq!((t.s("stage"), t.s("reviewerOn")), ("approval", "claude/sonnet"), "{}", t.at("failure"));
    let launches = calls(&w);
    assert!(launches.iter().any(|c| c.starts_with("agent start") && c.contains("--kind claude") && c.contains("--model sonnet") && c.contains("--effort medium")), "{launches:?}");
}

#[test]
fn reciprocal_peers_exhaust_once_without_looping_or_escalating_on_an_outage() {
    let w = cut_world("30", json!({ "worker.agent": "claude/sonnet", "fallback.corresponding": true })); pairs();
    w.script(json!({ "worker": [{ "ends": "claude-usage-limit-made" }, { "ends": "codex-unauthorized" }, { "status": "working" }] }));
    let id = w.add("Both providers unavailable"); delegate(&id); let t = settle(&id);
    assert_eq!(t.s("stage"), "failed");
    assert_eq!(starts(&w), vec!["claude", "codex"]);
    assert!(t.at("fallbacks").s("worker").contains("was on it already"));
    assert_eq!(t.arr("handovers").len(), 1);
}

#[test]
fn task_agents_cannot_initialize_or_replace_the_correspondence_policy() {
    let w = world(); w.script(json!({ "worker": [{ "status": "working" }] }));
    let id = w.add("Worker"); delegate(&id); let t = settle(&id);
    common::pilot::in_pane(&w, t.at("link").s("pane"));
    assert!(common::pilot::refused(&["models", "catalog", "--init", "--profile", "corresponding"]).contains("human"));
    assert!(common::pilot::refused(&["models", "catalog", "--import", "/tmp/policy.toml"]).contains("human"));
    common::pilot::in_pane(&w, "");
}
