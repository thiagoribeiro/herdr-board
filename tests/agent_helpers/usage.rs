//! Native subagent usage, copied history, delayed transcripts and token-limit enforcement over the whole task family.
use crate::common;
use common::*;
use sw::json::*;
use std::path::{Path, PathBuf};

fn write(file: &Path, rows: &[Value]) {
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, rows.iter().map(|r| format!("{r}\n")).collect::<String>()).unwrap();
}
fn claude(id: &str, input: i64) -> Value {
    json!({ "type": "assistant", "message": { "id": id, "usage": { "input_tokens": input, "output_tokens": 10 } } })
}
fn codex(input: i64, last: i64, at: i64) -> Value {
    json!({ "timestamp": sw::clock::iso(at), "type": "event_msg", "payload": { "type": "token_count", "info": {
        "total_token_usage": { "input_tokens": input, "output_tokens": 10 }, "last_token_usage": { "input_tokens": last, "output_tokens": 10 } } } })
}
fn meta(id: &str, parent: &str, at: i64) -> Value {
    json!({ "type": "session_meta", "timestamp": sw::clock::iso(at), "payload": { "id": id, "parent_thread_id": parent } })
}
fn spawn(id: &str) -> Value { json!({ "type": "event_msg", "payload": { "item": { "type": "SubAgentActivity", "kind": "started", "agent_thread_id": id } } }) }
fn task_file(w: &World, file: &Path, kind: &str) -> String {
    let id = w.add("Measured family");
    sw::store::tasks::patch(&id, |t| { t.put("agent", kind); t.put("session", "root"); t.put("transcript", file.display().to_string()); }).unwrap();
    id
}
fn count(id: &str) -> Obj { sw::limits::spent::count(id, "agent", 2000).unwrap().o("spent").unwrap().clone() }

#[test]
fn claude_children_and_grandchildren_count_once_even_with_copied_replies_and_repeated_reads() {
    let w = world(); let root = w.base.join("root.jsonl");
    let child = root.with_extension("").join("subagents/agent-first.jsonl");
    let grandchild = child.with_extension("").join("subagents/agent-second.jsonl");
    write(&root, &[claude("root", 100)]);
    write(&child, &[claude("root", 100), claude("child", 1000)]);
    write(&grandchild, &[claude("grandchild", 50)]);
    let id = task_file(&w, &root, "claude");
    let first = count(&id);
    assert_eq!(first.i("total"), 1180);
    assert_eq!(first.at("metrics").i("requests"), 3);
    assert_eq!((first.i("sessions"), first.i("subagents"), first.b("partial")), (3, 2, false));
    let again = count(&id);
    assert_eq!((again.i("total"), again.at("metrics").i("requests"), again.i("subagents")), (1180, 3, 2), "a poll counts nothing twice");
    assert!(sw::limits::spent::counted(&task(&id), "agent").is_none(), "an unchanged family does not rewrite counters");
    // Usage written after a reviewer has closed or a worker switched must still be discovered.
    let next = w.base.join("next.jsonl"); write(&next, &[claude("next", 20)]);
    sw::store::tasks::patch(&id, |t| { t.put("session", "next"); t.put("transcript", next.display().to_string()); }).unwrap();
    write(&grandchild, &[claude("grandchild", 50), claude("late", 200)]);
    assert_eq!(count(&id).i("total"), 1420);
}

#[test]
fn missing_claude_children_are_partial_then_count_when_their_transcript_arrives() {
    let w = world(); let root = w.base.join("root.jsonl");
    write(&root, &[claude("root", 100), json!({ "type": "user", "message": { "content": [
        { "type": "tool_result", "content": "Async agent launched successfully.\nagentId: delayed" } ] } })]);
    let id = task_file(&w, &root, "claude");
    assert!(count(&id).b("partial"));
    write(&root.with_extension("").join("subagents/agent-delayed.jsonl"), &[claude("child", 500)]);
    let done = count(&id);
    assert_eq!((done.i("total"), done.b("partial")), (620, false));
}

#[test]
fn codex_children_verify_parentage_and_copied_history_is_not_billed_again() {
    let w = world();
    let dir = PathBuf::from(std::env::var("CODEX_HOME").unwrap()).join("sessions/2026");
    let root = dir.join("rollout-root.jsonl");
    let child = dir.join("rollout-child.jsonl");
    let inherited = codex(1000, 1000, 1000);
    write(&root, &[meta("root", "", 500), inherited.clone(), spawn("child"), spawn("unrelated")]);
    write(&child, &[meta("child", "root", 2000), inherited, codex(1500, 500, 3000), spawn("grandchild")]);
    write(&dir.join("rollout-grandchild.jsonl"), &[meta("grandchild", "child", 4000), codex(200, 200, 5000)]);
    write(&dir.join("rollout-unrelated.jsonl"), &[meta("unrelated", "someone-else", 4000), codex(90000, 90000, 5000)]);
    let id = task_file(&w, &root, "codex");
    let counted = count(&id);
    assert_eq!(counted.i("total"), 1720);
    assert_eq!((counted.i("subagents"), counted.b("partial")), (2, true));
    assert_eq!((count(&id).i("total"), count(&id).i("subagents")), (1720, 2));
    // A child file that was not yet available stays pending, even after the main conversation stops growing.
    write(&dir.join("rollout-unrelated.jsonl"), &[meta("unrelated", "root", 4000), codex(100, 100, 5000)]);
    assert_eq!((count(&id).i("total"), count(&id).b("partial")), (1830, false));
}

#[test]
fn a_childs_usage_can_stop_the_task_before_check_or_review() {
    let w = world(); let root = w.base.join("root.jsonl");
    write(&root, &[claude("root", 100)]);
    write(&root.with_extension("").join("subagents/agent-large.jsonl"), &[claude("child", 10000)]);
    let id = task_file(&w, &root, "claude");
    sw::store::tasks::patch(&id, |t| { t.obj_mut("settings").put("limit.tokens", "5k"); }).unwrap();
    assert_eq!(sw::limits::reached::tokens_over(&id, "agent"), Some(("5k".into(), 10120.0)));
}

#[test]
fn unsupported_and_not_yet_saved_sessions_are_visible_as_incomplete_measurements() {
    let w = world(); let root = w.base.join("absent.jsonl");
    let id = task_file(&w, &root, "claude");
    assert!(count(&id).b("partial"));
    write(&root, &[claude("first", 100)]);
    assert!(!count(&id).b("partial"));
    sw::store::tasks::patch(&id, |t| { t.put("agent", "opencode"); t.put("session", "unmeasured"); t.del("transcript"); }).unwrap();
    let spent = count(&id);
    assert_eq!(spent.i("total"), 110);
    assert!(spent.b("partial") && spent.arr("owed").iter().any(|f| f.s("kind") == "opencode"));
}

#[test]
fn a_saved_transcript_without_usage_is_unknown_and_an_unreadable_history_stays_partial() {
    let w = world(); let root = w.base.join("root.jsonl");
    let started = json!({ "type": "user", "message": { "content": "Starting" } });
    write(&root, &[started.clone()]);
    let id = task_file(&w, &root, "claude");
    assert!(count(&id).b("partial"), "a file without provider usage does not prove zero spend");
    write(&root, &[started, claude("root", 100)]);
    assert!(!count(&id).b("partial"));
    std::fs::remove_file(&root).unwrap();
    let lost = count(&id);
    assert_eq!(lost.i("total"), 110, "previous evidence is retained");
    assert!(lost.b("partial"));
}

#[test]
fn a_symlink_to_another_sessions_children_is_not_attributed_to_this_task() {
    let w = world(); let root = w.base.join("root.jsonl");
    write(&root, &[claude("root", 100)]);
    let other = w.base.join("other/subagents");
    write(&other.join("agent-unrelated.jsonl"), &[claude("other", 90000)]);
    std::fs::create_dir_all(root.with_extension("")).unwrap();
    std::os::unix::fs::symlink(&other, root.with_extension("").join("subagents")).unwrap();
    let id = task_file(&w, &root, "claude");
    let spent = count(&id);
    assert_eq!((spent.i("total"), spent.i("subagents")), (110, 0));
    assert!(spent.b("partial"));
}

#[test]
fn old_aliases_of_the_same_transcript_do_not_charge_the_family_twice() {
    let w = world(); let root = w.base.join("root.jsonl"); let alias = w.base.join("alias.jsonl");
    write(&root, &[claude("root", 100)]);
    std::os::unix::fs::symlink(&root, &alias).unwrap();
    let id = task_file(&w, &root, "claude");
    let mut old = sw::agents::read_tokens::claude_tokens(&alias, &Value::Null).unwrap();
    old["kind"] = json!("claude"); old["session"] = json!("root");
    sw::store::tasks::patch(&id, |t| t.put("spent", json!({ "files": { alias.display().to_string(): old } }))).unwrap();
    let spent = count(&id);
    assert_eq!((spent.i("total"), spent.i("sessions"), spent.b("partial")), (110, 1, false));
    assert!(spent.at("files").at(&alias.display().to_string()).is_null());
}
