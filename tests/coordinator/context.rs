//! Context checkpoint hints across programs and safe continuity of an autopilot coordinator's own conversation.
use crate::common;
use common::*;
use sw::json::*;
use sw::coordinator::{co::Co, context, state, usage};
use std::path::Path;

fn write(file: &Path, rows: &[Value]) {
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, rows.iter().map(|r| format!("{r}\n")).collect::<String>()).unwrap();
}
fn wrapped(co: &Co) -> Value {
    let words = context::wrap(co, &json!({ "id": "request", "context": {} }), "continue");
    serde_json::from_str(words.split(context::OPEN).nth(1).unwrap().strip_suffix(context::CLOSE).unwrap()).unwrap()
}

#[test]
fn codex_context_is_the_last_request_not_cumulative_tokens_and_follows_compaction() {
    let w = world(); let co = Co::top(&w.socket);
    let file = std::path::PathBuf::from(std::env::var("CODEX_HOME").unwrap()).join("sessions/rollout-root.jsonl");
    let event = |total, last| json!({ "type": "event_msg", "payload": { "type": "token_count", "info": {
        "total_token_usage": { "input_tokens": total, "output_tokens": 10 }, "last_token_usage": { "input_tokens": last } } } });
    let rows = vec![json!({ "type": "turn_context", "payload": { "model": "chosen-model" } }), event(1000000, 180000)];
    write(&file, &rows);
    state::change(&co, |c| { c.put("session", "root"); c.put("run", json!({ "kind": "codex" })); true }).unwrap();
    usage::count(&co);
    assert_eq!(state::get(&co).i("contextTokens"), 180000);
    assert_eq!(state::get(&co).s("observedModel"), "chosen-model");
    assert!(wrapped(&co).b("checkpointSuggested"));
    let mut rows = rows; rows.push(event(1020000, 20000)); write(&file, &rows);
    usage::count(&co);
    assert_eq!(state::get(&co).i("contextTokens"), 20000);
    assert!(!wrapped(&co).b("checkpointSuggested"));
    assert_eq!(usage::count(&co).i("input"), 1020000);
    state::change(&co, |c| { c.put("session", "new-without-a-file"); true }).unwrap();
    assert!(wrapped(&co).at("contextTokens").is_null(), "an older session's context is not the new session's");
    assert!(!wrapped(&co).b("checkpointSuggested"));
}

#[test]
fn pi_context_includes_cached_input_and_retains_its_observed_model() {
    let w = world(); let file = w.base.join("pi.jsonl");
    write(&file, &[json!({ "type": "message", "message": { "role": "assistant", "model": "pi-model", "usage": {
        "input": 10000, "cacheRead": 160000, "cacheWrite": 10000, "output": 400 } } })]);
    let c = sw::agents::read_tokens::pi_tokens(&file, &Value::Null).unwrap();
    assert_eq!((c.i("lastContext"), c.i("n"), c.s("model")), (180000, 180400, "pi-model"));
    assert_eq!(sw::agents::read_tokens::pi_tokens(&file, &c).unwrap(), c);
}

#[test]
fn claude_context_ignores_replayed_old_replies_and_incomplete_usage_parts() {
    let w = world(); let file = w.base.join("claude.jsonl");
    let reply = |id, input, output| json!({ "type": "assistant", "message": { "id": id, "usage": { "input_tokens": input, "output_tokens": output } } });
    write(&file, &[reply("first", 180000, 10), reply("first", 0, 20)]);
    let before = sw::agents::read_tokens::claude_tokens(&file, &Value::Null).unwrap();
    assert_eq!(before.i("lastContext"), 180000);
    write(&file, &[reply("first", 180000, 10), reply("first", 0, 20), reply("latest", 20000, 10), reply("first", 180000, 20), reply("first", 180000, 20)]);
    let after = sw::agents::read_tokens::claude_tokens(&file, &before).unwrap();
    assert_eq!((after.i("lastContext"), after.s("id"), after.at("metrics").i("requests")), (20000, "latest", 2));
}

#[test]
fn coordinator_usage_includes_children_without_mistaking_their_context_for_its_own() {
    let w = world(); let co = Co::top(&w.socket);
    let root = std::path::PathBuf::from(std::env::var("CLAUDE_CONFIG_DIR").unwrap()).join("projects/x/root.jsonl");
    let reply = |id, input| json!({ "type": "assistant", "message": { "id": id, "usage": { "input_tokens": input, "output_tokens": 10 } } });
    write(&root, &[reply("root", 100)]);
    write(&root.with_extension("").join("subagents/agent-child.jsonl"), &[reply("child", 180000)]);
    state::change(&co, |c| { c.put("session", "root"); c.put("run", json!({ "kind": "claude" })); true }).unwrap();
    assert_eq!(usage::count(&co).i("input"), 180100);
    assert_eq!(state::get(&co).at("usageCoverage").i("subagents"), 1);
    assert_eq!(state::get(&co).i("contextTokens"), 100);
    assert!(!wrapped(&co).b("checkpointSuggested"));
    state::change(&co, |c| { c.put("session", "not-saved-yet"); true }).unwrap(); usage::count(&co);
    assert!(state::get(&co).at("usageCoverage").b("partial"));
    assert!(wrapped(&co).at("contextTokens").is_null());
}

#[test]
fn autopilot_can_save_and_checkpoint_only_its_own_conversation_without_widening_permissions() {
    use common::pilot::*;
    let w = pilot_world(); let co = proj(&w);
    w.script(json!({ "coordinator": [{ "status": "working" }, { "status": "working" }] }));
    ok(&["autopilot", "on", "--project", "proj", "--goal", "Docs", "--autopilot.every", "none"]); tick(&w);
    in_pane(&w, &pane_of(&co));
    assert!(sw::autopilot::turn::autopilot_turn(&co));
    let memory = json!({ "decisions": ["Keep original authorization"], "constraints": [], "questions": ["Keep pending work"] }).to_string();
    assert!(refused(&["coordinator", "memory", &memory]).contains("own conversation"));
    assert!(refused(&["coordinator", "checkpoint", "--machine"]).contains("own conversation"));
    ok(&["coordinator", "memory", &memory, "--project", "proj"]);
    assert!(refused(&["set", "autopilot.approve", "yes"]).contains("cannot change settings"));
    assert!(refused(&["coordinator", "new", "--project", "proj"]).contains("human's"));
    let before = state::get(&co).unwrap();
    state::change(&co, |c| { c.put("contextTokens", 180000); c.put("contextSession", c.t("session")); true }).unwrap();
    ok(&["coordinator", "checkpoint", "--project", "proj"]); drain();
    let after = state::get(&co).unwrap();
    assert_ne!(before.at("session"), after.at("session"));
    for k in ["kind", "model", "effort", "access", "fast"] { assert_eq!(before.at("run").at(k), after.at("run").at(k)); }
    assert_eq!(after.at("memory").at("content").arr("questions"), &[json!("Keep pending work")]);
    assert_eq!(after.arr("checkpoints").len(), 1);
    assert!(after.at("contextTokens").is_null());
    assert!(sw::autopilot::record::is_on(&co));
    assert!(state::get(Co::top(&w.socket)).at("memory").is_null());
    in_pane(&w, "");
}
