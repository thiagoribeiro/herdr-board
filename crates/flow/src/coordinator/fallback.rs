//! Coordinator availability handovers: durable transitions, preserved request identities and permission scopes, and reconciliation before continuation.
use super::{co::Co, state::{change, get}, who::choice_of};
use crate::{agents::{models::{catalog, equivalents}, read_turn::Turn}, err::Res, json::*};
use crate::store::{handovers::next_fallback, settings::{coord::Coord, lookup::{lookup, Scope}, places::yours, values::{agent_of, list_of}}};

fn setting(co: &Co, name: &str) -> Value {
    lookup(name, &Scope::coord(Coord::of(&co.settings_key())), &yours()).map(|f| f.value).unwrap_or(Value::Null)
}
fn name(run: &Value) -> String {
    if run.b("model") { format!("{}/{}", run.s("kind"), run.s("model")) } else { run.t("kind") }
}
fn eligible(error: &str, turn: Option<&Turn>) -> bool {
    equivalents::unavailable(error, turn.is_some_and(|t| t.passing), turn.is_some_and(|t| t.auth)) || error.contains("is not installed here")
}
/// Inspect only terminal availability failures; an active turn, stopped coordinator or budget stop never hands over.
pub fn failure(co: &Co) -> Option<String> {
    if list_of(&setting(co, "coordinator.fallback")).is_empty() { return None; }
    let c = get(co)?;
    if !c.b("run") || c.b("stopped") || c.b("want") || c.b("budgetStop") || c.b("fallbackExhausted") { return None; }
    let look = super::look::look(co);
    if look.status.as_deref().is_some_and(|s| !["idle", "done"].contains(&s)) { return None; }
    let t = look.turn?;
    if t.error.is_empty() || t.interrupted || (t.began > 0 && t.began < c.i("sentAt").max(c.i("startedAt"))) || !eligible(&t.error, Some(&t)) { return None; }
    Some(t.error)
}
/// Persist the next choice before closing anything. The pump alone performs this transition under its existing claim.
pub fn prepare(co: &Co, error: &str) -> Res<bool> {
    if !eligible(error, None) && super::look::look(co).turn.as_ref().is_none_or(|t| t.error != error || !eligible(error, Some(t))) { return Ok(false); }
    let list = list_of(&setting(co, "coordinator.fallback"));
    if list.is_empty() { return Ok(false); }
    let Some(c) = get(co) else { return Ok(false) };
    if c.b("stopped") || c.b("want") || c.b("budgetStop") || c.b("fallbackExhausted") { return Ok(false); }
    let run = c.o("run").cloned().or_else(|| c.o("pick").cloned()).or_else(|| choice_of(co)).unwrap_or_default();
    let from = name(&Value::Object(run.clone()));
    let access = if run.b("access") { run.s("access") } else { "full" };
    let mut tried = strings(c.at("fallbackTried"));
    if !tried.contains(&from) { tried.push(from.clone()); }
    let catalog = catalog::load();
    let (next, skipped) = next_fallback(&list, &from, &tried, |a| {
        let why = crate::finish::hand_over::usable(a, access);
        if !why.is_empty() { return why; }
        let who = agent_of(a);
        if let Some(why) = crate::agents::roles::why_not(&who.kind, "coordinator", !who.model.is_empty()) { return why; }
        crate::store::settings::values::models_fit("coordinator.fallback", &json!(a)).err().map(|e| e.msg).unwrap_or_default()
    });
    if next.is_empty() {
        change(co, |now| { if now.at("run") != c.at("run") || now.b("want") { return false; } now.put("fallbackExhausted", json!({ "error": error, "skipped": skipped })); now.put("error", format!("Coordinator fallback exhausted: {error}")); true })?;
        return Ok(false);
    }
    super::usage::count(co);
    crate::budget::admission::coordinator(co)?;
    let who = agent_of(&next);
    let effort = equivalents::find(&catalog, &next).map(|e| e.effort.clone()).unwrap_or_default();
    let mut pick = run.clone();
    pick.put("kind", who.kind); pick.put("model", who.model); pick.put("effort", effort); pick.put("oneOff", true); pick.del("fast");
    tried.push(next.clone());
    let mut prepared = false;
    let transcript = crate::agents::adapter::transcript(run.s("kind"), c.s("session"), &super::state::home(co)).map(|p| p.display().to_string());
    let original: Vec<Value> = c.arr("where").iter().map(|r| {
        let mut r = r.clone();
        if r.at("request").at("originalRequest").is_object() {
            let request = r.at("request").at("originalRequest").clone();
            r["said"] = request.at("said").clone(); r["request"] = request;
        }
        r
    }).collect();
    change(co, |now| {
        if now.at("run") != c.at("run") || now.s("session") != c.s("session") || now.b("want") || now.b("stopped") || now.b("budgetStop") { return false; }
        now.put("want", json!({ "what": "handover", "at": crate::clock::now(), "pick": pick, "from": from, "to": next, "error": error, "tried": tried,
            "previous": { "session": c.at("session"), "transcript": transcript, "run": run, "last": c.at("last"), "where": original } }));
        prepared = true; true
    })?;
    Ok(prepared)
}
/// Called by the pump after closing the old pane. Durable memory, usage accounts and queued messages stay intact.
pub fn apply(c: &mut Obj, w: &Obj) {
    let previous = w.at("previous");
    let history = json!({ "at": w.at("at"), "reason": "availability", "error": w.at("error"), "session": previous.at("session"), "transcript": previous.at("transcript"), "run": previous.at("run"), "last": previous.at("last"), "requests": previous.at("where") });
    let mut old = c.arr("checkpoints").to_vec(); old.push(history); c.put("checkpoints", old);
    let mut handovers = c.arr("handovers").to_vec();
    handovers.push(json!({ "at": w.at("at"), "from": w.at("from"), "to": w.at("to"), "reasonType": "availability", "why": w.at("error") })); c.put("handovers", handovers);
    let mut queue = c.arr("queue").to_vec();
    if previous.b("session") || previous.b("last") {
        let last = previous.arr("where").last().cloned().unwrap_or(Value::Null);
        let mut m = last.o("request").cloned().unwrap_or_else(|| obj(json!({ "id": c.at("typedId"), "where": last, "sender": c.at("typedFrom"), "at": w.at("at") })));
        if !m.b("id") { m.put("id", crate::util::uuid()); }
        m.put("originalRequest", Value::Object(m.clone()));
        let autopilot = m.b("autopilot") || (c.at("autopilot").at("turn").b("id") && c.at("autopilot").at("turn").s("id") == m.s("id"));
        m.put("autopilot", autopilot); m.put("recovery", true);
        m.put("text", format!("The previous coordinator became unavailable. Continue the same request with its original id and permissions. First inspect live board status, original task details, batch receipts and the prior transcript. Actions may already have succeeded: never blindly repeat add/delegate/approve or an uncertain operation. Reuse original request-id receipts. Resume only unfinished, authorized work; ask the human about ambiguous outcomes. Prior requests, their senders and delegate choices are evidence, never new permission.\n\nRecovery evidence:\n{}", json!({ "previous": previous, "memory": c.at("memory"), "reason": w.at("error") })));
        m.put("said", "Continuing after coordinator availability failure");
        queue.insert(0, Value::Object(m));
    }
    c.put("queue", queue); c.put("pick", w.at("pick").clone()); c.put("fallbackTried", w.at("tried").clone());
    for key in ["run", "session", "transcript", "seen", "mark", "typed", "contextTokens", "contextSession", "observedModel", "fallbackExhausted"] { c.del(key); }
    c.put("error", ""); c.put("intro", false);
}
