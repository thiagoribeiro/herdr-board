//! Shared-budget admission before native agent prompts, runtime stops and measured helper accounting.
use crate::common;
use common::*;
use sw::{budget, json::*};
use super::world_budget;

#[test]
fn queued_work_does_not_start_until_an_ended_owners_reservation_is_reconciled() {
    let w = world_budget();
    w.script(json!({ "worker": [{ "status": "working" }, { "status": "working" }, { "status": "working" }] }));
    let first = w.add("First"); delegate(&first); assert_eq!(settle(&first).s("stage"), "working");
    let second = w.add("Second"); delegate(&second);
    // Its 300-token slice is admissible; a third task cannot reuse either reservation.
    assert_eq!(settle(&second).s("stage"), "working");
    let third = w.add("Third"); delegate(&third); assert_eq!(settle(&third).s("stage"), "queued");
    assert!(sw::shown::queue::words(&task(&third)).contains("shared budget"));
    sw::actions::talk::stop(&first).unwrap();
    budget::observe(&format!("task:{first}"), "proj", None, true).unwrap();
    sw::actions::start::start_queued(&w.socket); assert_eq!(task(&third).s("stage"), "queued");
    budget::settle(&format!("task:{first}"), 0).unwrap();
    sw::actions::start::start_queued(&w.socket); assert_eq!(settle(&third).s("stage"), "working");
}

#[test]
fn observed_exhaustion_stops_native_work_and_reply_does_not_bypass_the_shared_cap() {
    let w = world_budget(); w.script(json!({ "worker": [{ "status": "working" }] }));
    let id = w.add("Long native work"); delegate(&id); let t = settle(&id); assert_eq!(t.s("stage"), "working");
    let file = w.base.join("conversation.jsonl");
    std::fs::write(&file, format!("{}\n", json!({ "type": "assistant", "message": { "id": "request", "usage": { "input_tokens": 1100, "output_tokens": 10 }, "stop_reason": "end_turn", "content": [{ "type": "text", "text": "Working" }] } }))).unwrap();
    sw::store::tasks::patch(&id, |t| { t.put("transcript", file.display().to_string()); t.del("spent"); }).unwrap();
    sw::clock::advance(3000);
    budget::runtime::tick(&sw::flow::live::board_for(&w.socket), &w.socket);
    let stopped = task(&id);
    assert_eq!(stopped.s("stage"), "stopped");
    assert!(stopped.b("budgetStop") && stopped.b("requireApproval"));
    assert_eq!(row(&id).s("state"), "blocked");
    assert_eq!(budget::status().unwrap().at("global").i("used"), 1110);
    assert!(sw::actions::talk::tell(&id, "Continue", false).is_err());
    assert_eq!(task(&id).s("stage"), "stopped");
}

#[test]
fn helper_usage_includes_cache_and_an_unknown_result_retains_its_reservation() {
    let _w = world_budget();
    let claim = budget::helpers::begin("summary:one", "proj", "claude").unwrap();
    let out = json!({ "result": "original result", "usage": { "input_tokens": 10, "cache_read_input_tokens": 100, "output_tokens": 20 } }).to_string();
    assert_eq!(budget::helpers::finish(claim, &out), "original result");
    assert_eq!(budget::status().unwrap().at("global").i("used"), 130);
    let claim = budget::helpers::begin("watcher", "", "claude").unwrap();
    budget::helpers::finish(claim, "unmeasured response");
    let status = budget::status().unwrap();
    assert!(status.at("owners").as_object().unwrap().values().any(|o| o.b("uncertain")));
    assert_eq!(status.at("global").i("reserved"), 700);
    assert!(budget::helpers::begin("summary:two", "proj", "claude").is_err());
}

#[test]
fn task_agents_cannot_raise_shared_caps_or_reconcile_their_own_spending() {
    let w = world(); w.script(json!({ "worker": [{ "status": "working" }] }));
    let id = w.add("Ordinary worker"); delegate(&id); let t = settle(&id);
    common::pilot::in_pane(&w, t.at("link").s("pane"));
    assert!(common::pilot::refused(&["set", "budget.tokens", "none"]).contains("only the human"));
    assert!(common::pilot::refused(&["budget", "settle", &format!("task:{id}"), "--used", "0"]).contains("only the human"));
    common::pilot::in_pane(&w, "");
}

#[test]
fn the_panel_distinguishes_shared_observed_spending_from_promised_allowance() {
    let w = world_budget(); let id = w.add("Budget visibility");
    budget::acquire(&format!("task:{id}"), "proj", "agent", Some(0)).unwrap();
    let mut board = common::board::open(&w.root, 80); board.cols = 180;
    let text = board.text();
    assert!(text.contains("Shared budget"), "{text}");
    assert!(text.contains("reserved") && text.contains("limit"), "{text}");
}

#[test]
fn watcher_usage_is_charged_to_its_project_when_the_machine_cap_is_disabled() {
    let w = world_with(Opts { repo: true, settings: json!({ "watcher.agent": "claude", "review.when": "never", "budget.reserve": "700", "budget.reviewPercent": 0 }), ..Default::default() });
    w.script(json!({ "worker": [{ "status": "working", "screen": "Editing one project\n" }] }));
    let id = w.add("Only this project"); delegate(&id); settle(&id);
    sw::store::settings::change::set_value("budget.projectTokens", &json!("1k"), &sw::store::settings::lookup::Scope::repo("proj")).unwrap();
    let mut rows = sw::flow::live::board_for(&w.socket);
    let mut another = rows[0].clone(); another.put("repo", "another"); another.put("title", "Another project"); another.put("id", "another/1"); rows.push(another);
    let calls = std::cell::Cell::new(0);
    sw::watcher::watcher_reset();
    sw::watcher::watch_tick_with(&rows, sw::clock::now(), Some(0.0), &|cmd, _, done| {
        calls.set(calls.get() + 1);
        let prompt = cmd.stdin.unwrap_or_default();
        assert!(prompt.contains("Only this project") && !prompt.contains("Another project"));
        assert!(cmd.args.windows(2).any(|w| w == ["--output-format", "json"]));
        done(0, json!({ "result": "[{\"n\":1,\"msg\":\"Editing\",\"progress\":20}]", "usage": { "input_tokens": 10, "cache_read_input_tokens": 100, "output_tokens": 20 } }).to_string(), String::new());
    });
    assert_eq!(calls.get(), 1);
    let s = budget::status().unwrap();
    assert!(s.at("global").at("limit").is_null());
    assert_eq!(s.at("projects").at("proj").i("used"), 130);
}
