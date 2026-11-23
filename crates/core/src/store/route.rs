//! A task's route (docs/models.md#catalog-and-routing): the entry of the catalog of models it was given, kept on the
//! task as `route` { entry (the one on it now), first, reason, by, at, chain: [{ name, agent, effort, cost }],
//! escalated: [{ from, to, at }] }. The settings lookup reads it as a place of its own, just under the task's own
//! settings (`routed: padrao — "why"`): its entry's program and effort are the task's `worker.agent` and
//! `worker.effort`, the entries it escalates to its `worker.fallback`. A program you name for the task
//! (`--worker.agent`) wins over all of it. The chain is copied when the route is given, so a later change to the
//! catalog does not move a task under way.
use crate::agents::models::catalog::{load, Catalog};
use crate::clock::now;
use crate::err::Res;
use crate::json::*;
use crate::store::settings::lookup::{lookup, setting, setting_text, Scope};
use crate::store::settings::places::yours;
use crate::store::settings::values::{models_fit, nearest, NONE};
use crate::store::tasks::{one_line, update, Change, Task};

/// The human named the program for this task: that wins, and a route is not read.
pub fn human_chose(t: &Task) -> bool { t.o("settings").map(|s| s.contains_key("worker.agent")).unwrap_or(false) }
/// The route as kept on a task, for the entry named; refused when the catalog has no such entry, or its model is one
/// its program does not take. An entry further down its chain whose model is refused is left out of the chain.
pub fn record(c: &Catalog, name: &str, reason: &str, by: &str) -> Res<Value> {
    let Some(e) = c.get(name) else {
        let why = c.problems.iter().find(|p| p.starts_with(&format!("entry {name}:"))).map(|p| format!(" Its entry is refused: {p}.")).unwrap_or_default();
        let near = nearest(name, &c.names()).map(|n| format!(" Did you mean {n}?")).unwrap_or_default();
        crate::refuse!("No entry \"{name}\" in the catalog of models.{why}{near} It has: {} (herdr-board models catalog).", if c.entries.is_empty() { "none".to_string() } else { c.names().join(", ") });
    };
    models_fit("worker.agent", &json!(e.agent)).map_err(|x| crate::err::Error::refused(format!("The catalog's entry {}: {}", e.name, x.msg)))?;
    let chain: Vec<Value> = c.chain(&e.name).into_iter().enumerate().filter(|(i, x)| *i == 0 || models_fit("worker.agent", &json!(x.agent)).is_ok())
        .map(|(_, x)| json!({ "name": x.name, "agent": x.agent, "effort": x.effort, "cost": x.cost })).collect();
    Ok(json!({ "entry": e.name, "first": e.name, "reason": one_line(reason, 200), "by": by, "at": now(), "chain": chain, "escalated": [] }))
}
/// Checked before anything is added: a refused `--route` adds nothing.
pub fn checked(name: &str) -> Res<()> { record(&load(), name, "", "").map(|_| ()) }
/// Give a task its route (`--route`). What it says after the task's own line.
pub fn choose(id: &str, name: &str, reason: &str, by: &str) -> Res<String> {
    let r = record(&load(), name, reason, by)?;
    let t = update(id, |t| { if human_chose(t) { return Ok(Change::Keep); } t.put("route", r.clone()); Ok(Change::Write) })?;
    if human_chose(&t) { return Ok(format!(" (--route {name} not used: worker.agent was named for this task, and the model you name wins)")); }
    Ok(format!(", routed to {}", r.s("entry")))
}
/// The entry of the chain that is on the task now.
pub fn current(r: &Value) -> Option<&Value> { r.arr("chain").iter().find(|x| x.s("name") == r.s("entry")) }
fn after_current(r: &Value) -> Vec<&Value> { r.arr("chain").iter().skip_while(|x| x.s("name") != r.s("entry")).skip(1).collect() }
/// Where its values came from, as `show` says it: `routed: padrao — "small refactor, 2 files"`.
pub fn from_words(r: &Value) -> String {
    let why = if !r.s("reason").is_empty() { format!(" — \"{}\"", r.s("reason")) } else if r.s("by") == "default" { " (route.default)".into() } else { String::new() };
    let up = if r.s("entry") != r.s("first") { format!(", escalated from {}", r.s("first")) } else { String::new() };
    format!("routed: {}{up}{why}", r.s("entry"))
}
/// The settings its route says, and where they came from; None for a task with no route, or one whose program you named.
pub fn place_of(t: &Task) -> Option<(Obj, String)> {
    if t.b("attemptOf") || human_chose(t) { return None; }
    let r = t.at("route");
    let cur = current(r)?;
    let mut o = Obj::new();
    o.put("worker.agent", cur.at("agent").clone());
    o.put("worker.effort", if cur.s("effort").is_empty() { json!("default") } else { cur.at("effort").clone() });
    let mut rest: Vec<Value> = vec![];
    for x in after_current(r) { if x.at("agent") != cur.at("agent") && !rest.contains(x.at("agent")) { rest.push(x.at("agent").clone()); } }
    if !rest.is_empty() { o.put("worker.fallback", Value::Array(rest)); }
    Some((o, from_words(r)))
}
/// A hand-over to `next`: when it is the program of an entry further down the chain, that entry is on the task now,
/// and its effort with it.
pub fn on_handover(t: &mut Task, next: &str) {
    if !t.at("route").is_object() { return; }
    let r = t.at("route").clone();
    let Some(to) = after_current(&r).into_iter().find(|x| x.s("agent") == next).map(|x| x.t("name")) else { return };
    let o = t.obj_mut("route");
    let mut ups = o.arr("escalated").to_vec();
    ups.push(json!({ "from": r.s("entry"), "to": to, "at": now() }));
    o.put("escalated", ups);
    o.put("entry", to);
}
/// Delegated: a task with a route starts again on its first entry; one with none gets `route.default`, under
/// `route.mode coordinator` or when the task (or the automation that made it) says it itself. No model is asked.
pub fn at_delegate(t: &mut Task) {
    if t.at("route").is_object() { let first = t.at("route").t("first"); if !first.is_empty() { t.obj_mut("route").put("entry", first); } return; }
    if human_chose(t) || t.b("attemptOf") { return; }
    let Ok(f) = lookup("route.default", &Scope::task(t), &yours()) else { return };
    let name = text(&f.value);
    if name.is_empty() || name == NONE || (setting_text("route.mode", Some(t)) != "coordinator" && f.place != "task") { return; }
    let by = if t.b("automation") { "automation" } else { "default" };
    if let Ok(r) = record(&load(), &name, "", by) { t.put("route", r); }
}
/// A routed task's work failed review for the `rounds + 1`th time: does the next entry of its chain take over now,
/// before `review.rounds` is used up? Only while there is one to take over.
pub fn escalates_early(t: &Task, rounds: i64) -> bool {
    if !t.at("route").is_object() || human_chose(t) || t.b("attemptOf") { return false; }
    let r = t.at("route");
    let on = t.t("on");
    let left = after_current(r).iter().any(|x| x.s("agent") != on && !t.arr("tried").iter().any(|y| text(y) == x.t("agent")));
    left && rounds + 1 >= setting("route.escalateAfter", Some(t)).as_i64().unwrap_or(1)
}
