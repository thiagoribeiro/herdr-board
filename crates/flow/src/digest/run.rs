//! Detached, globally bounded summary calls with cross-process claims, retry backoff and rejection of late results.
use super::*;
use crate::agents::adapter::{one_shot, OneShot};
use crate::store::{files::{marker, marker_alive, with_lock_wait}, settings::lookup::setting_text, settings::values::agent_of, tasks::{update, Change}};
pub type Done = Box<dyn FnOnce(i32, String, String) + Send>;
pub type Run<'a> = &'a dyn Fn(OneShot, &[(&str, &str)], Done);
const LEASE_MS: i64 = 60000;

pub fn tick(tasks: &[Obj]) {
    // Commands and task runners exit after housekeeping. The summary must own a detached runner, not their thread.
    if crate::runner::inline() || setting_text("summary.agent", None) == "none" { return; }
    let now = crate::clock::now();
    let agent = setting_text("summary.agent", None);
    if !tasks.iter().any(|t| ready(t) && cached(t).is_null() &&
        (t.at("digestAttempt").s("key") != key(t) || t.at("digestAttempt").s("agent") != agent || (t.at("digestAttempt").i("attempts") < 3 && now >= t.at("digestAttempt").i("retryAt")))) { return; }
    let _ = with_lock_wait("summary-job", 0, || {
        let path = crate::store::files::data_dir().join("summary-job.json");
        let old = crate::store::files::read_json(&path).unwrap_or(Value::Null);
        if marker_alive(&old) && now >= old.i("at") && now - old.i("at") < 120000 { return Ok(()); }
        let pid = crate::housekeeping::spawn_loose(&["run", "summaries", "-"]);
        if pid > 0 { crate::store::files::write_json(&path, &json!({ "pid": pid, "start": crate::store::files::proc_start(pid as i64), "at": now }))?; }
        Ok(())
    });
}
/// Keep the dedicated process alive until its helper delivers or times out. No UI/keeper thread is blocked.
pub fn job() {
    let tasks = crate::store::tasks::list(Default::default());
    let (tx, rx) = std::sync::mpsc::channel();
    let launched = std::cell::Cell::new(false);
    tick_with(&tasks, &|cmd, env, done| {
        launched.set(true);
        let tx = tx.clone();
        crate::watcher::run_helper(cmd, env, move |code, out, err| { done(code, out, err); let _ = tx.send(()); });
    });
    if launched.get() { let _ = rx.recv_timeout(std::time::Duration::from_secs(55)); }
}
pub fn tick_with(tasks: &[Obj], run: Run) {
    let agent = setting_text("summary.agent", None);
    if agent == "none" { return; }
    let who = agent_of(&agent);
    if crate::agents::models::allow::start_refusal(&who.kind).is_some() { return; }
    if !["claude", "pi"].contains(&who.kind.as_str()) { return; }
    let now = crate::clock::now();
    for seen in tasks {
        if !ready(seen) || !cached(seen).is_null() { continue; }
        let id = seen.t("id");
        let mut claimed = None;
        // One summary at a time, including when several board/keeper processes tick together.
        let locked = with_lock_wait("summary-call", 0, || {
            let path = crate::store::files::data_dir().join("summary-call.json");
            let old = crate::store::files::read_json(&path).unwrap_or(Value::Null);
            if now - old.i("at") < LEASE_MS && marker_alive(&old) { return Ok(()); }
            let _ = update(&id, |t| {
                if !ready(t) || !cached(t).is_null() { return Ok(Change::Keep); }
                let key = key(t);
                let same = t.at("digestAttempt").s("key") == key && t.at("digestAttempt").s("agent") == agent;
                let attempts = if same { t.at("digestAttempt").i("attempts") } else { 0 };
                if attempts >= 3 { return Ok(Change::Keep); }
                if t.at("digestAttempt").s("key") == key && now < t.at("digestAttempt").i("retryAt") { return Ok(Change::Keep); }
                let mut mark = marker(); mark.put("at", now); mark.put("key", key.clone());
                mark.put("plain", same && t.at("digestAttempt").b("plain"));
                mark.put("agent", agent.clone()); mark.put("retryAt", now + 300000);
                mark.put("attempts", attempts + 1);
                t.put("digestAttempt", mark.clone());
                claimed = Some((t.clone(), key));
                crate::store::files::write_json(&path, &Value::Object(mark))?;
                Ok(Change::Write)
            });
            Ok(())
        });
        if locked.is_err() { return; }
        let Some((task, expected)) = claimed else { continue };
        // An already structured delivery needs no model to reword it.
        if let Some(body) = prompt::parse(task.s("summary")).filter(|_| !task.b("review")) {
            let _ = update(&id, |t| { if key(t) != expected { return Ok(Change::Keep); } t.put("digest", result(t, body)); t.obj_mut("digest").put("by", "agent"); Ok(Change::Write) });
            release(&expected, now); continue;
        }
        let input = source(&task);
        // Never silently truncate requirements or evidence. Oversized evidence remains available in full.
        if input.to_string().len() > 64000 {
            failed(&id, &expected, "Evidence exceeds 64 KB; inspect the original delivery", now);
            release(&expected, now); continue;
        }
        let lean = !task.at("digestAttempt").b("plain");
        let Some(mut cmd) = one_shot(&who.kind, &who.model, "", &prompt::prompt(&input), prompt::SYSTEM, lean) else {
            failed(&id, &expected, "Summary program has no one-shot mode", now); release(&expected, now); continue;
        };
        if who.kind == "claude" { cmd.args.extend(["--output-format".into(), "json".into()]); }
        run(cmd, &[], Box::new(move |code, out, err| {
            if code != 0 && lean && crate::re!(r"(?i)unknown (option|argument)|unrecognized|unexpected argument").is_match(&err) {
                let _ = update(&id, |t| { if t.at("digestAttempt").i("at") != now { return Ok(Change::Keep); } t.obj_mut("digestAttempt").put("plain", true); Ok(Change::Write) });
            }
            let envelope = serde_json::from_str::<Value>(&out).ok();
            let raw = envelope.as_ref().filter(|v| v.at("result").is_string() && v.at("usage").is_object());
            let _ = update(&id, |t| {
                let total = t.obj_mut("digestUsage");
                total.put("calls", total.i("calls") + 1);
                if let Some(v) = raw { for k in ["input_tokens", "cache_creation_input_tokens", "cache_read_input_tokens", "output_tokens"] { total.put(k, total.f(k) + v.at("usage").f(k)); } }
                else { total.put("unmeasuredCalls", total.i("unmeasuredCalls") + 1); }
                Ok(Change::Write)
            });
            if let Some(v) = raw {
                let _ = update(&id, |t| {
                    if t.at("digestAttempt").i("at") != now { return Ok(Change::Keep); }
                    t.obj_mut("digestAttempt").put("usage", v.at("usage").clone());
                    t.obj_mut("digestAttempt").put("durationMs", (crate::clock::now() - now).max(0));
                    Ok(Change::Write)
                });
            }
            let body = (code == 0).then(|| prompt::parse(raw.map(|v| v.s("result")).unwrap_or(&out))).flatten();
            if let Some(body) = body {
                let _ = update(&id, |t| {
                    if key(t) != expected || t.at("digestAttempt").i("at") != now || !ready(t) { return Ok(Change::Keep); }
                    t.put("digest", result(t, body));
                    t.obj_mut("digestAttempt").put("finishedAt", crate::clock::now());
                    Ok(Change::Write)
                });
            } else { failed(&id, &expected, "Summary unavailable; original evidence retained", now); }
            release(&expected, now);
        }));
        return;
    }
}
fn failed(id: &str, key: &str, reason: &str, began: i64) {
    let _ = update(id, |t| {
        if t.at("digestAttempt").s("key") != key || t.at("digestAttempt").i("at") != began { return Ok(Change::Keep); }
        t.obj_mut("digestAttempt").put("error", reason);
        t.obj_mut("digestAttempt").put("finishedAt", crate::clock::now());
        Ok(Change::Write)
    });
}
fn release(key: &str, began: i64) {
    let _ = with_lock_wait("summary-call", 0, || {
        let p = crate::store::files::data_dir().join("summary-call.json");
        let v = crate::store::files::read_json(&p).unwrap_or(Value::Null);
        if v.s("key") == key && v.i("at") == began { let _ = std::fs::remove_file(p); }
        Ok(())
    });
}
