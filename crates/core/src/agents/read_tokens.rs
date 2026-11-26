//! The tokens a saved conversation has used so far, one counter per agent program: everything the model read and
//! everything it wrote, every request of the conversation added up, as the provider counts it (what a request read
//! again from the cache too). A conversation file only grows, so it is read on from where the last count stopped:
//! `state` is what the counter gave back the time before ({ at: the byte it read to, n: the tokens so far, … }).
use crate::json::*;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// The whole lines written since byte `at`, and where they end. A last line still being written waits.
pub(super) fn grown(file: &Path, at: u64) -> std::io::Result<(Vec<Value>, u64)> {
    let size = std::fs::metadata(file)?.len();
    let at = if size < at { 0 } else { at };
    if size == at { return Ok((vec![], at)); }
    let mut f = std::fs::File::open(file)?;
    f.seek(SeekFrom::Start(at))?;
    let mut buf = vec![0u8; (size - at) as usize];
    f.read_exact(&mut buf)?;
    let end = buf.iter().rposition(|b| *b == b'\n').map(|i| i + 1).unwrap_or(0);
    let rows = String::from_utf8_lossy(&buf[..end]).split('\n').filter(|l| !l.is_empty()).filter_map(|l| serde_json::from_str(l).ok()).collect();
    Ok((rows, at + end as u64))
}
fn from(file: &Path, state: &Value) -> Obj {
    let size = std::fs::metadata(file).map(|m| m.len()).unwrap_or(0) as f64;
    if state.is_object() && size >= state.f("at") { state.as_object().cloned().unwrap_or_default() } else { Obj::new() }
}
fn partial(s: &Obj) -> bool { s.b("partial") || (s.f("n") > 0.0 && !s.at("metrics").is_object()) }
/// Claude Code writes one line per part of a reply, each with the whole reply's use: a reply counts once, by its id.
pub fn claude_tokens(file: &Path, state: &Value) -> std::io::Result<Value> {
    let s = from(file, state);
    let (mut n, mut id, mut id_n) = (s.f("n"), s.t("id"), s.f("idN"));
    let mut metrics = s.at("metrics").clone();
    let mut seen = s.o("seen").cloned().unwrap_or_default();
    let mut model = s.t("model");
    let mut last_context = s.get("lastContext").cloned().unwrap_or_else(|| seen.get(&id)
        .map(|p| json!(p.f("input") + p.f("cacheWrite") + p.f("cacheRead"))).unwrap_or(Value::Null));
    let (rows, at) = grown(file, s.f("at") as u64)?;
    for e in rows {
        let u = e.at("message").at("usage");
        if e.s("type") != "assistant" || e.b("isApiErrorMessage") || !u.is_object() { continue; }
        let v = u.f("input_tokens") + u.f("cache_creation_input_tokens") + u.f("cache_read_input_tokens") + u.f("output_tokens");
        let mid = e.at("message").t("id");
        let mut parts = json!({ "input": u.f("input_tokens"), "cacheWrite": u.f("cache_creation_input_tokens"), "cacheRead": u.f("cache_read_input_tokens"), "output": u.f("output_tokens") });
        let prior = seen.get(&mid).cloned().unwrap_or(Value::Null);
        for k in ["input", "cacheWrite", "cacheRead", "output"] { parts[k] = json!(parts.f(k).max(prior.f(k))); }
        let latest = prior.is_null() || mid == id;
        if latest {
            last_context = json!(parts.f("input") + parts.f("cacheWrite") + parts.f("cacheRead"));
            if e.at("message").b("model") { model = e.at("message").t("model"); }
        }
        add_metrics(&mut metrics, &parts, &prior, prior.is_null());
        if !mid.is_empty() {
            let old_total: f64 = if prior.is_null() && mid == id { id_n } else { ["input", "cacheWrite", "cacheRead", "output"].iter().map(|k| prior.f(k)).sum() };
            let total: f64 = ["input", "cacheWrite", "cacheRead", "output"].iter().map(|k| parts.f(k)).sum();
            n += (total - old_total).max(0.0);
            seen.insert(mid.clone(), parts);
        } else { n += v; }
        if latest { id = mid; id_n = v; }
    }
    Ok(json!({ "at": at, "n": n, "id": id, "idN": id_n, "metrics": metrics, "seen": seen, "model": model, "lastContext": last_context, "partial": partial(&s) }))
}
fn add_metrics(out: &mut Value, now: &Value, before: &Value, request: bool) {
    if !out.is_object() { *out = json!({}); }
    for k in ["input", "cacheWrite", "cacheRead", "output"] { out[k] = json!(out.f(k) + (now.f(k) - before.f(k)).max(0.0)); }
    out["requests"] = json!(out.i("requests") + i64::from(request));
}
/// Codex writes the conversation's running total after every request. A total that went down began again.
pub fn codex_tokens(file: &Path, state: &Value) -> std::io::Result<Value> {
    let s = from(file, state);
    let (mut n, mut last) = (s.f("n"), s.f("last"));
    let mut metrics = s.at("metrics").clone();
    let mut last_parts = s.at("lastParts").clone();
    let mut context = s.get("lastContext").cloned().unwrap_or(Value::Null);
    let mut model = s.t("model");
    let mut fork_at = s.i("forkAt");
    let (rows, at) = grown(file, s.f("at") as u64)?;
    for e in rows {
        let p = e.at("payload");
        if e.s("type") == "session_meta" && (p.b("parent_thread_id") || p.at("source").at("subagent").is_object()) {
            fork_at = super::read_turn::when(e.at("timestamp"));
        }
        if e.s("type") == "turn_context" && p.b("model") { model = p.t("model"); }
        let u = p.at("info").at("total_token_usage");
        if e.s("type") != "event_msg" || p.s("type") != "token_count" || !u.is_object() { continue; }
        // Context is the latest request's input, never the conversation's cumulative billable input.
        if let Some(n) = p.at("info").at("last_token_usage").at("input_tokens").as_u64() { context = json!(n); }
        let v = u.f("input_tokens") + u.f("output_tokens");
        let parts = json!({ "input": (u.f("input_tokens") - u.f("cached_input_tokens")).max(0.0), "cacheRead": u.f("cached_input_tokens"), "output": u.f("output_tokens") });
        // Forked rollouts can copy the parent's earlier usage events. They establish the baseline, not new spend.
        let event_at = super::read_turn::when(e.at("timestamp"));
        if fork_at > 0 && event_at > 0 && event_at < fork_at { last = v; last_parts = parts; continue; }
        if v != last { add_metrics(&mut metrics, &parts, if v < last { &Value::Null } else { &last_parts }, true); }
        last_parts = parts;
        n += if v >= last { v - last } else { v };
        last = v;
    }
    Ok(json!({ "at": at, "n": n, "last": last, "metrics": metrics, "lastParts": last_parts, "model": model, "lastContext": context, "forkAt": fork_at, "partial": partial(&s) }))
}
/// pi writes each reply once, with its own use.
pub fn pi_tokens(file: &Path, state: &Value) -> std::io::Result<Value> {
    let s = from(file, state);
    let mut n = s.f("n");
    let mut metrics = s.at("metrics").clone();
    let mut context = s.get("lastContext").cloned().unwrap_or(Value::Null);
    let mut model = s.t("model");
    let (rows, at) = grown(file, s.f("at") as u64)?;
    for e in rows {
        let m = e.at("message");
        if e.s("type") == "message" && m.s("role") == "assistant" && m.at("usage").is_object() {
            let u = m.at("usage");
            if m.b("model") { model = m.t("model"); }
            if u.at("input").is_number() { context = json!(u.f("input") + u.f("cacheRead") + u.f("cacheWrite")); }
            add_metrics(&mut metrics, &json!({ "input": u.f("input"), "cacheRead": u.f("cacheRead"), "cacheWrite": u.f("cacheWrite"), "output": u.f("output") }), &Value::Null, true);
            n += u.f("input") + u.f("cacheRead") + u.f("cacheWrite") + u.f("output");
        }
    }
    Ok(json!({ "at": at, "n": n, "metrics": metrics, "model": model, "lastContext": context, "partial": partial(&s) }))
}
/// This program's counter, where it has one.
pub fn tokens(kind: &str, file: &Path, state: &Value) -> Option<Value> {
    match kind { "claude" => claude_tokens(file, state).ok(), "codex" => codex_tokens(file, state).ok(), "pi" => pi_tokens(file, state).ok(), _ => None }
}
