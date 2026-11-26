//! Budget admission for task and coordinator sessions, before program launch or prompt delivery.
use crate::{coordinator::{co::Co, state}, err::Res, json::*, store::tasks::{get, Task}};
pub fn owner(t: &Task) -> String { format!("task:{}", t.s("id")) }
pub fn coordinator_owner(co: &Co) -> String { format!("coordinator:{}", co.key()) }
pub fn total(t: &Task) -> Option<u64> {
    let s = t.at("spent");
    (!s.b("partial") && s.at("files").as_object().is_some_and(|f| !f.is_empty())).then(|| super::tokens(s.at("total"))).flatten()
}
fn existing(owner: &str) -> bool {
    let state = super::status().unwrap_or(Value::Null);
    let o = state.at("owners").at(owner);
    o.b("active") && !o.b("uncertain") && o.i("held") > 0 && crate::clock::now().saturating_sub(o.i("observedAt").max(o.i("startedAt"))) < super::runtime::UNKNOWN_MS
}
pub fn task(t: &Task, role: &str) -> Res<()> {
    let repo = t.so("repo").unwrap_or_else(|| t.s("id").split('/').next().unwrap_or(""));
    if !super::config::enabled(repo) && !super::exists() { return Ok(()); }
    let kind = if role == "reviewer" { t.at("link").at("reviewer").so("kind").map(String::from).unwrap_or_else(|| crate::store::settings::values::agent_of(&crate::store::settings::lookup::setting_text("reviewer.agent", Some(t))).kind) } else { t.so("agent").map(String::from).unwrap_or_else(|| crate::store::settings::values::agent_of(&crate::store::settings::lookup::setting_text("worker.agent", Some(t))).kind) };
    if super::config::enabled(repo) && !crate::limits::spent::can_count(&kind) { crate::fail!("Shared budgets cannot measure {kind}; choose a supported program or change the budget explicitly"); }
    let counted = crate::limits::spent::count(t.s("id"), role, 2000).unwrap_or_else(|| t.clone());
    let fresh = !t.b("startedAgentAt") && t.i("reviewRound") == 0 && !t.at("spent").at("files").is_object();
    let owner = owner(t);
    let reconciled = if t.s("stage") == "stopped" { super::status().ok().map(|s| s.at("owners").at(&owner).clone()).filter(|o| !o.b("active") && !o.b("uncertain") && o.at("manualFloor").is_number() && o.i("reconciledAt") >= t.i("stoppedAt")) } else { None };
    let total = total(&counted).or_else(|| fresh.then_some(0)).filter(|_| t.s("stage") != "stopped" || super::runtime::ended_evidence(&counted) || reconciled.is_some())
        .or_else(|| reconciled.as_ref().and_then(|o| o.at("last").as_u64()));
    if total.is_none() && existing(&owner) { return Ok(()); } // its reserved call may not have written usage yet
    super::acquire_measured(&owner, repo, role, total, counted.at("spent").at("files"))?;
    Ok(())
}
pub fn coordinator(co: &Co) -> Res<()> {
    let repo = co.repo().unwrap_or("");
    if !super::config::enabled(repo) && !super::exists() { return Ok(()); }
    let c = state::get(co).unwrap_or_default();
    let fresh = !c.b("mark") && c.arr("checkpoints").is_empty() && c.at("usageFiles").as_object().is_none_or(|f| f.is_empty());
    let kind = c.at("run").s("kind");
    if super::config::enabled(repo) && !crate::limits::spent::can_count(kind) { crate::fail!("Shared budgets cannot measure coordinator program {kind}"); }
    let totals = crate::coordinator::usage::count(co);
    let c = state::get(co).unwrap_or_default();
    let measured = !c.at("usageCoverage").b("partial") || fresh;
    let total = measured.then(|| ["input", "cacheWrite", "cacheRead", "output"].iter().map(|k| totals.f(k)).sum::<f64>() as u64);
    let owner = coordinator_owner(co);
    if total.is_none() && existing(&owner) { return Ok(()); }
    super::acquire_measured(&owner, repo, "coordinator", total, c.at("usageFiles"))?;
    let _ = state::change(co, |c| { if !c.b("budgetStop") { return false; } c.del("budgetStop"); true });
    Ok(())
}
/// Every board-generated prompt passes this gate, including queued messages and comparison/review briefs.
pub fn pane(api: &crate::herdr::Herdr, pane: &str) -> Res<()> {
    if !super::tracking() { return Ok(()); }
    let socket = api.socket_path();
    if let Some(co) = crate::autopilot::turn::coordinator_of(pane, &socket) { return coordinator(&co); }
    let tasks = crate::store::tasks::list(crate::store::tasks::ListOpts { attempts: true, ..Default::default() });
    if let Some((t, role)) = crate::flow::pane_owner::whose(tasks, &socket, pane) {
        let t = get(t.s("id")).unwrap_or(t);
        let result = task(&t, role);
        if let Err(e) = &result { super::runtime::task_stop(&t, &e.msg); }
        return result;
    }
    if super::config::global().is_some() { crate::fail!("No shared-budget owner was found for this prompt's pane"); }
    Ok(())
}
