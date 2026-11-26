//! Availability-only correspondence policy for task workers and reviewers, with explicit fallback lists taking precedence.
use crate::{agents::models::{catalog::{load, Entry}, equivalents}, json::*};
use super::{settings::{lookup::{lookup, Scope}, places::yours, values::list_of}, tasks::Task};

pub struct Choices { pub agents: Vec<String>, pub entries: Vec<Entry>, pub catalog: bool }
pub fn choices(t: &Task, role: &str, on: &str, settings: &Obj, outage: bool) -> Choices {
    let key = format!("{role}.fallback");
    let manual = || Choices { agents: list_of(settings.at(&key)), entries: vec![], catalog: false };
    if !outage || settings.at("fallback.corresponding") != &json!(true) { return manual(); }
    let found = lookup(&key, &Scope::task(t), &yours()).ok();
    if found.is_some_and(|f| !["builtin", "routed"].contains(&f.place.as_str())) { return manual(); }
    let c = load();
    let routed = (role == "worker").then(|| t.at("route").s("entry")).and_then(|n| c.get(n)).filter(|e| e.agent == on);
    let source = routed.or_else(|| equivalents::find(&c, on));
    let entries = source.map(|e| equivalents::candidates(&c, &e.name)).unwrap_or_default();
    Choices { agents: entries.iter().map(|e| e.agent.clone()).collect(), entries, catalog: true }
}
fn key(role: &str) -> &str { if role == "worker" { "workerEquivalent" } else { "reviewerEquivalent" } }
pub fn select(t: &mut Task, role: &str, agent: &str, entries: &[Entry]) {
    match entries.iter().find(|e| e.agent == agent) {
        Some(e) => t.put(key(role), e.json()),
        None => t.del(key(role)),
    }
}
/// An explicit effort on the task wins. Otherwise a corresponding model keeps its own declared effort.
pub fn effort(t: &Task, role: &str, on: &str) -> Option<String> {
    if t.at("settings").has(&format!("{role}.effort")) { return None; }
    let e = t.at(key(role));
    (e.s("agent") == on && e.b("effort")).then(|| e.t("effort"))
}
pub fn log(t: &mut Task, role: &str, from: &str, to: &str, why: &str) -> Value {
    let mut h = super::handovers::log_handover(t, if role == "worker" { "worker" } else { "reviewer" }, from, to, why);
    let e = t.at(key(role));
    if e.s("agent") != to { return h; }
    h["reasonType"] = json!("availability"); h["correspondingEntry"] = e.at("name").clone(); h["family"] = e.at("family").clone();
    let mut all = t.arr("handovers").to_vec();
    if let Some(last) = all.last_mut() { *last = h.clone(); }
    t.put("handovers", all);
    h
}
