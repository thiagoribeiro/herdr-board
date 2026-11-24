//! Coordinator economy preserves authorization, original requirements and evidence while avoiding repeated helper turns.
use crate::common;
use common::*;
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use sw::json::*;
use sw::store::tasks::patch;

fn body() -> Value { json!({ "result": "Agent reports the change is implemented", "changes": ["Adjusted validation"], "verified": ["Check passed"], "limitations": ["No browser check"], "questions": [], "observed": [] }) }
fn delivery(w: &World) -> String {
    let id = w.add("Validate input");
    patch(&id, |t| { t.put("stage", "done"); t.put("summary", "Implemented input validation. Check passed; no browser check."); t.put("prHead", "abc1234"); }).unwrap();
    id
}
fn rows(id: &str) -> Vec<Obj> { vec![task(id)] }
fn cli(words: &[&str]) -> sw::err::Res<(Value, String)> {
    let a = sw::cli::flags::parse(&words.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    sw::cli::tasks::command(words[0], &a)
}

#[test]
fn unchanged_delivery_is_reused_but_new_evidence_invalidates_the_cache() {
    let w = world_with(Opts { settings: json!({ "summary.agent": "claude/haiku" }), ..Default::default() });
    let id = delivery(&w);
    let calls = Cell::new(0);
    let run = |cmd: sw::agents::adapter::OneShot, _: &[(&str, &str)], done: sw::digest::run::Done| {
        calls.set(calls.get() + 1);
        assert!(cmd.args.iter().any(|s| s == "--no-session-persistence"));
        assert!(cmd.args.windows(2).any(|w| w[0] == "--tools" && w[1].is_empty()));
        done(0, json!({ "result": body().to_string(), "usage": { "input_tokens": 100, "output_tokens": 30 } }).to_string(), String::new());
    };
    sw::digest::run::tick_with(&rows(&id), &run);
    assert_eq!(sw::digest::cached(&task(&id)), body());
    assert_eq!(task(&id).at("digestAttempt").at("usage").i("input_tokens"), 100);
    patch(&id, |t| { t.put("at", sw::clock::now() + 1000); t.put("prMerged", true); t.put("watch", json!({ "progress": 90 })); }).unwrap();
    sw::digest::run::tick_with(&rows(&id), &run);
    assert_eq!(calls.get(), 1, "polls and workflow transitions do not regenerate evidence summaries");
    patch(&id, |t| { t.put("review", json!({ "head": "abc1234", "meets": false, "text": "A defect remains", "findings": [{ "severity": "major", "subject": "Missing validation" }] })); }).unwrap();
    assert!(sw::digest::cached(&task(&id)).is_null());
    sw::digest::run::tick_with(&rows(&id), &run);
    assert_eq!(calls.get(), 2);
    assert!(sw::digest::cached(&task(&id)).arr("observed")[0].as_str().unwrap().contains("Independent review"));
    assert!(task(&id).b("summary"), "original evidence is retained");
}

#[test]
fn delayed_results_are_rejected_and_global_claims_prevent_duplicate_calls() {
    let w = world_with(Opts { settings: json!({ "summary.agent": "claude/haiku" }), ..Default::default() });
    let id = delivery(&w);
    let pending = RefCell::new(None);
    sw::digest::run::tick_with(&rows(&id), &|_, _, done| *pending.borrow_mut() = Some(done));
    sw::digest::run::tick_with(&rows(&id), &|_, _, _| panic!("duplicate helper"));
    patch(&id, |t| t.put("summary", "New delivery: tests failed")).unwrap();
    pending.borrow_mut().take().unwrap()(0, body().to_string(), String::new());
    assert!(sw::digest::cached(&task(&id)).is_null(), "an older result cannot replace changed evidence");
    sw::digest::run::tick_with(&rows(&id), &|_, _, done| done(0, body().to_string(), String::new()));
    assert_eq!(sw::digest::cached(&task(&id)), body());
}

#[test]
fn invalid_summaries_preserve_originals_and_stop_after_three_attempts() {
    let w = world_with(Opts { settings: json!({ "summary.agent": "claude/haiku" }), ..Default::default() });
    let id = delivery(&w);
    let calls = Cell::new(0);
    sw::clock::set(Some(1700000000000));
    for _ in 0..6 {
        sw::digest::run::tick_with(&rows(&id), &|_, _, done| { calls.set(calls.get() + 1); done(0, "{\"result\":\"Made up\"}".into(), String::new()); });
        sw::clock::advance(301000);
    }
    assert_eq!(calls.get(), 3);
    assert!(sw::digest::cached(&task(&id)).is_null());
    assert!(task(&id).at("digestAttempt").b("error"));
    assert_eq!(task(&id).s("summary"), "Implemented input validation. Check passed; no browser check.");
}

#[test]
fn each_request_keeps_its_original_selection_and_only_explicit_task_context() {
    let w = world();
    let first = delivery(&w);
    let other = w.add("Unrelated task");
    let where_ = json!({ "repo": "proj", "root": w.root_str(), "task": first, "delegate": true });
    let ctx = sw::coordinator::context::capture(&where_, "continue this", &Value::Null);
    assert_eq!(ctx.arr("tasks").len(), 1);
    assert_eq!(ctx.arr("tasks")[0].s("id"), first);
    let message = json!({ "id": "request-1", "said": "continue this", "where": where_, "context": ctx });
    let co = sw::coordinator::co::Co::top(&w.socket);
    let wrapped = sw::coordinator::context::wrap(&co, &message, "continue this");
    assert!(wrapped.contains(&first) && !wrapped.contains(&other));
    assert_eq!(sw::coordinator::context::bare(&wrapped), "continue this");
    let explicit = sw::coordinator::context::relevant(&format!("compare {first} and {other}"), "");
    assert_eq!(explicit.len(), 2);
}

#[test]
fn batch_receipts_are_idempotent_and_validate_ids_before_mutating() {
    let w = world();
    let a = w.add("First"); let b = w.add("Second");
    assert!(cli(&["batch", "archive", &a, "proj/999", "--request-id", "invalid"]).is_err());
    assert!(!task(&a).b("archivedAt"));
    let (receipt, _) = cli(&["batch", "archive", &a, &b, "--request-id", "one"]).unwrap();
    assert!(receipt.b("complete") && receipt.b("ok"));
    assert_eq!(receipt.arr("results").len(), 2);
    cli(&["restore", &a]).unwrap();
    let (again, _) = cli(&["batch", "archive", &a, &b, "--request-id", "one"]).unwrap();
    assert!(again.b("replayed"));
    assert_eq!(receipt.arr("results")[0].at("message"), again.arr("results")[0].at("message"));
    assert!(!task(&a).b("archivedAt"), "reusing a receipt never repeats a side effect");
    assert!(cli(&["batch", "stop", &a, "--request-id", "one"]).is_err());
}

#[test]
fn status_is_live_while_cached_delivery_is_shown_without_waking_a_coordinator() {
    let w = world(); let id = delivery(&w);
    patch(&id, |t| t.put("digest", sw::digest::result(t, body()))).unwrap();
    let co = sw::coordinator::co::Co::top(&w.socket);
    let before = sw::coordinator::state::get(&co);
    let messages = sw::digest::messages(&co);
    assert_eq!(messages.len(), 1);
    assert!(messages[0].text.contains(&id) && messages[0].text.contains("No browser check"));
    assert_eq!(sw::coordinator::state::get(&co), before);
    patch(&id, |t| t.put("stage", "approval")).unwrap();
    let (v, _) = cli(&["status", &id]).unwrap();
    assert_eq!(v[0].s("owner"), "human");
    assert_eq!(v[0].at("summary"), &body());
    let (v, _) = cli(&["status"]).unwrap(); assert_eq!(v.as_array().unwrap().len(), 1);
}

#[test]
fn usage_separates_cache_and_new_tokens_and_deduplicates_replayed_replies() {
    let w = world(); let file = w.base.join("usage.jsonl");
    let entry = |id: &str, out: i64| json!({ "type": "assistant", "message": { "id": id, "usage": { "input_tokens": 10, "cache_creation_input_tokens": 20, "cache_read_input_tokens": 1000, "output_tokens": out } } });
    let contents = [entry("a", 5), entry("a", 8), entry("b", 4), entry("a", 5), entry("a", 8)].iter().map(|v| format!("{v}\n")).collect::<String>();
    std::fs::write(&file, contents).unwrap();
    let count = sw::agents::read_tokens::claude_tokens(&file, &Value::Null).unwrap();
    assert_eq!(count.f("n"), 2072.0);
    assert_eq!(count.at("metrics"), &json!({ "input": 20.0, "cacheWrite": 40.0, "cacheRead": 2000.0, "output": 12.0, "requests": 2 }));
    assert_eq!(sw::agents::read_tokens::claude_tokens(&file, &count).unwrap(), count);
    let id = w.add("Measured without a limit");
    patch(&id, |t| { t.put("agent", "claude"); t.put("session", "session"); t.put("transcript", file.display().to_string()); }).unwrap();
    let measured = sw::limits::spent::count(&id, "agent", 2000).unwrap();
    assert_eq!(measured.at("spent").at("metrics").i("requests"), 2);
}

#[test]
fn compact_rules_preserve_quality_and_authorization_and_memory_is_bounded() {
    let w = world(); let co = sw::coordinator::co::Co::top(&w.socket);
    let rules = sw::coordinator::rules::rules_of(&co, "/bin/board");
    assert!(rules.len() < 7000, "core instructions should remain small: {} bytes", rules.len());
    assert!(rules.len() * 3 < sw::coordinator::rules::reference_of(&co, "/bin/board").len(), "the core is at least three times smaller than the full reference");
    for rule in ["not human authorization", "Never raise or remove", "Independent review", "Never start polling", "coordinator help"] { assert!(rules.contains(rule), "{rule}"); }
    assert!(sw::coordinator::context::remember(&co, &json!({ "decisions": "missing arrays" })).is_err());
    sw::coordinator::context::remember(&co, &json!({ "decisions": ["Keep the model"], "constraints": ["Review independently"], "questions": [] })).unwrap();
    let words = sw::coordinator::context::wrap(&co, &json!({ "id": "later", "context": {} }), "continue");
    assert!(words.contains("Keep the model"));
    let help = sw::coordinator::rules::help(&co, "/bin/board", "fallback").unwrap();
    assert!(help.contains("worker.fallback") && help.contains("reviewer.fallback"));
    assert!(!help.contains("Automations:") && !help.contains("Several attempts at once"));
}

#[test]
fn report_edits_invalidate_the_cache_even_when_the_link_does_not_change() {
    let w = world(); let id = delivery(&w);
    let folder = sw::research::report_folder(&task(&id)); std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("report.md"), "First evidence").unwrap();
    patch(&id, |t| t.put("report", json!({ "file": "report.md", "link": "same-link" }))).unwrap();
    let before = sw::digest::key(&task(&id));
    patch(&id, |t| t.put("digest", sw::digest::result(t, body()))).unwrap();
    std::fs::write(folder.join("report.md"), "New evidence").unwrap();
    assert_ne!(before, sw::digest::key(&task(&id)));
    assert!(sw::digest::cached(&task(&id)).is_null());
    assert_eq!(sw::digest::source(&task(&id)).at("reportDocument").s("text"), "New evidence");
    patch(&id, |t| t.put("report", json!({ "file": "../../outside" }))).unwrap();
    assert!(sw::digest::source(&task(&id)).at("reportDocument").b("unavailable"));
}

#[test]
fn the_real_pump_sends_context_but_chat_only_shows_human_words() {
    let w = crate::coordinator::coord_world();
    let id = w.add("Selected task");
    sw::coordinator::requests::send(&w.root, "continue this", false, &id, &w.socket).unwrap();
    sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); });
    let raw = w.state().arr("panes").iter().find(|p| p.at("agent").s("name").starts_with("sw-coordinator")).unwrap().at("agent").arr("prompts")[0].clone();
    assert!(raw.as_str().unwrap().contains(sw::coordinator::context::OPEN));
    assert!(raw.as_str().unwrap().contains(&id));
    let view = sw::coordinator::look::look(&w.socket);
    assert!(view.talk.iter().all(|s| !s.text.contains(sw::coordinator::context::OPEN)));
}

#[test]
fn reviewers_receive_requirements_beyond_the_old_character_cutoff() {
    let w = world(); let id = w.add("Complete requirements");
    patch(&id, |t| t.put("description", format!("{}\nLAST REQUIREMENT: preserve authorization", "a".repeat(4000)))).unwrap();
    let choices = Obj::new(); let t = task(&id);
    let brief = sw::finish::review_brief::review_brief(&t, &sw::finish::review_brief::Brief { text: "Done", change: None, to_land: false, ch: &choices, earlier: None, check: "" });
    assert!(brief.contains("LAST REQUIREMENT: preserve authorization"));
}

#[test]
fn checkpoint_preserves_the_running_choice_memory_pending_requests_and_history() {
    let w = crate::coordinator::coord_world();
    sw::coordinator::requests::send(&w.root, "first", false, "", &w.socket).unwrap();
    let drain = || sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); });
    drain();
    let before = sw::coordinator::state::get(&w.socket).unwrap();
    let co = sw::coordinator::co::Co::top(&w.socket);
    sw::coordinator::context::remember(&co, &json!({ "decisions": ["Keep the choice"], "constraints": [], "questions": [] })).unwrap();
    sw::coordinator::requests::send(&w.root, "queued", false, "", &w.socket).unwrap();
    sw::coordinator::context::remember(&co, &json!({ "decisions": ["Keep the choice"], "constraints": [], "questions": ["Answer queued"] })).unwrap();
    cli(&["coordinator", "checkpoint"]).unwrap(); drain();
    let after = sw::coordinator::state::get(&w.socket).unwrap();
    assert_ne!(before.at("session"), after.at("session"));
    for k in ["kind", "model", "effort"] { assert_eq!(before.at("run").at(k), after.at("run").at(k)); }
    assert_eq!(after.at("memory").at("content").arr("questions"), &[json!("Answer queued")]);
    assert_eq!(after.arr("checkpoints").len(), 1);
    assert!(after.arr("queue").is_empty());
    assert_eq!(after.s("last"), "queued");
}

#[test]
fn real_programs_summary_survives_the_originating_command() {
    use std::os::unix::{fs::PermissionsExt, process::CommandExt};
    let w = world_with(Opts { repo: true, settings: json!({ "summary.agent": "claude/haiku" }), ..Default::default() });
    let id = delivery(&w);
    let bin = w.base.join("bin"); std::fs::create_dir_all(&bin).unwrap();
    let helper = bin.join("claude");
    std::fs::write(&helper, "#!/bin/sh\ncat >/dev/null\ncat \"$SUMMARY_REPLY_FILE\"\n").unwrap();
    std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o755)).unwrap();
    let reply = w.base.join("reply.json");
    std::fs::write(&reply, json!({ "result": body().to_string(), "usage": { "input_tokens": 17, "output_tokens": 9 } }).to_string()).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_herdr-board"))
        .args(["housekeep"]).current_dir(&w.root).process_group(0)
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display())).env("HERDR_BOARD_INLINE", "0")
        .env("HERDR_SOCKET_PATH", "/nonexistent").env("SUMMARY_REPLY_FILE", &reply).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while sw::digest::cached(&task(&id)).is_null() || sw::store::files::data_dir().join("summary-call.json").exists() {
        assert!(std::time::Instant::now() < deadline, "summary runner lost its result after the originating command exited");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(sw::digest::cached(&task(&id)), body());
    assert_eq!(task(&id).at("digestUsage").i("input_tokens"), 17);
    assert_eq!(task(&id).at("digestUsage").i("calls"), 1);
    assert!(!sw::store::files::data_dir().join("summary-call.json").exists());
}
