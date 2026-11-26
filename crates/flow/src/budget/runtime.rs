//! Shared-budget polling: refill measured runs, pause exhausted work and retain uncertain reservations across failures.
use super::admission;
use crate::{coordinator::{co::Co, state}, json::*, store::tasks::{update_wait, Change, Task}};
pub(super) const UNKNOWN_MS: i64 = 120000;

pub fn task_stop(t: &Task, reason: &str) {
    if t.b("budgetStop") && t.s("stage") == "stopped" { return; }
    let _ = crate::actions::talk::board_stop(t.s("id"));
    let _ = update_wait(t.s("id"), 2000, |t| {
        if t.s("stage") != "stopped" { return Ok(Change::Keep); }
        t.put("budgetStop", json!({ "at": crate::clock::now(), "reason": reason, "owner": admission::owner(t) }));
        t.put("requireApproval", true); t.put("msg", format!("Shared budget: {reason}"));
        Ok(Change::Write)
    });
}
fn stale(o: &Value) -> bool { crate::clock::now().saturating_sub(o.i("observedAt").max(o.i("startedAt"))) >= UNKNOWN_MS }
pub(super) fn ended_evidence(t: &Task) -> bool {
    t.at("spent").at("files").as_object().is_some_and(|files| !files.is_empty() && files.iter().all(|(path, f)| {
        crate::agents::read_turn::read_turn(f.s("kind"), std::path::Path::new(path), crate::agents::read_turn::TAIL).is_some_and(|turn| turn.finished)
    }))
}
pub fn tick(rows: &[Task], socket: &str) {
    if !super::tracking() { return; }
    let key = format!("budget-tick-{}", crate::coordinator::rules_news::hash(socket));
    if !crate::store::marks::due(&key, 2000) { return; }
    let snapshot = super::status().unwrap_or(Value::Null);
    let mut all = rows.to_vec();
    for row in rows { for child in row.arr("tries").iter().filter_map(Value::as_object) { if !all.iter().any(|t| t.s("id") == child.s("id")) { all.push(child.clone()); } } }
    for seen in &all {
        let row = crate::store::tasks::get(seen.s("id")).unwrap_or_else(|| seen.clone());
        if row.b("host") || row.at("live").b("away") || !crate::herdr::cli::same_socket(row.at("link").s("socket"), socket) { continue; }
        let owner = admission::owner(&row);
        let registered = snapshot.at("owners").at(&owner);
        let active = ["working", "reviewing", "reviewneeds"].contains(&row.s("stage"));
        let repo = row.so("repo").unwrap_or_else(|| row.s("id").split('/').next().unwrap_or(""));
        if !registered.is_object() && !(active && super::config::enabled(repo)) { continue; }
        let role = if ["reviewing", "reviewneeds"].contains(&row.s("stage")) { "reviewer" } else { "agent" };
        let t = crate::limits::spent::count(row.s("id"), role, 300).unwrap_or_else(|| row.clone());
        let total = admission::total(&t).filter(|_| active || row.s("stage") != "stopped" || ended_evidence(&t));
        if active && total.is_none() && registered.b("active") && !stale(registered) { continue; }
        let _ = super::observe_measured(&owner, repo, total, !active, t.at("spent").at("files"));
        if active && super::config::enabled(repo) {
            if let Err(e) = super::acquire_measured(&owner, repo, role, total, t.at("spent").at("files")) {
                task_stop(&t, &e.msg);
                let _ = super::observe(&owner, repo, None, true); // cancellation can still have a final provider bill in flight
            }
        }
    }
    for co in state::all(socket) { coordinator_tick(&co, &snapshot); }
}
fn coordinator_tick(co: &Co, snapshot: &Value) {
    let owner = admission::coordinator_owner(co);
    let old = snapshot.at("owners").at(&owner);
    let repo = co.repo().unwrap_or("");
    if !old.is_object() { return; }
    let totals = crate::coordinator::usage::count(co);
    let Some(c) = state::get(co) else { return };
    let api = crate::herdr::Herdr::new(Some(co.socket.clone()));
    let Ok(status) = api.agent_get(c.s("pane")) else { return };
    let active = status.as_ref().is_some_and(|a| ["working", "blocked"].contains(&a.s("agent_status")));
    let measured = !c.at("usageCoverage").b("partial");
    let finished = active || c.at("usageFiles").as_object().is_some_and(|files| !files.is_empty() && files.iter().all(|(path, f)| {
        crate::agents::read_turn::read_turn(f.s("kind"), std::path::Path::new(path), crate::agents::read_turn::TAIL).is_some_and(|turn| turn.finished)
    }));
    let total = (measured && finished).then(|| ["input", "cacheWrite", "cacheRead", "output"].iter().map(|k| totals.f(k)).sum::<f64>() as u64);
    if active && total.is_none() && !stale(old) { return; }
    let _ = super::observe_measured(&owner, repo, total, !active, c.at("usageFiles"));
    if active && super::config::enabled(repo) {
        if let Err(e) = super::acquire_measured(&owner, repo, "coordinator", total, c.at("usageFiles")) {
            let kind = c.at("run").s("kind");
            let _ = api.agent_keys(c.s("pane"), &[crate::agents::installed::interrupt_key(kind)]);
            let _ = state::change(co, |c| { c.put("budgetStop", e.msg.clone()); c.put("error", format!("Shared budget: {}", e.msg)); true });
            let _ = super::observe(&owner, repo, None, true);
        }
    }
}
