//! Budget ledger transitions: reservations share one balance, survive crashes and carry across period boundaries.
use super::config;
use crate::{err::Res, json::*};

pub fn new() -> Value {
    let now = crate::clock::now();
    json!({ "version": 1, "window": { "from": now, "until": now.saturating_add(config::period()), "periodMs": config::period() }, "used": 0, "projects": {}, "owners": {}, "history": [] })
}
pub fn validate(s: &Value) -> Res<()> {
    if s.i("version") != 1 || !s.at("owners").is_object() || !s.at("projects").is_object() || s.at("used").as_u64().is_none() {
        crate::fail!("Invalid shared budget ledger; it was preserved, not reset");
    }
    if s.at("window").i("until") <= s.at("window").i("from") || !s.at("history").is_array() || s.at("projects").as_object().unwrap().values().any(|n| n.as_u64().is_none()) { crate::fail!("Invalid shared budget period or spending; the ledger was preserved"); }
    if s.get("sources").is_some_and(|v| !v.is_object() || v.as_object().unwrap().values().any(|n| n.as_u64().is_none())) { crate::fail!("Invalid shared budget evidence; the ledger was preserved"); }
    for o in s.at("owners").as_object().unwrap().values() {
        if !o.is_object() || o.at("last").as_u64().is_none() || o.at("held").as_u64().is_none() || !o.at("active").is_boolean() || !o.at("uncertain").is_boolean() {
            crate::fail!("Invalid shared budget owner; inspect its original evidence");
        }
    }
    Ok(())
}
fn n(v: &Value, k: &str) -> u64 { v.at(k).as_u64().unwrap_or(0) }
pub fn window(s: &mut Value, now: i64) {
    let until = s.at("window").i("until");
    if until > 0 && now < until { return; } // a clock going backwards grants no new allowance
    if until > 0 {
        let mut history = s.arr("history").to_vec();
        history.push(json!({ "window": s.at("window"), "used": s.at("used"), "projects": s.at("projects") }));
        history.drain(..history.len().saturating_sub(30)); s["history"] = json!(history);
    }
    let period = config::period();
    let from = if until > 0 { until + (now - until).max(0) / period * period } else { now };
    s["window"] = json!({ "from": from, "until": from.saturating_add(period), "periodMs": period });
    s["used"] = json!(0); s["projects"] = json!({});
    // Owner baselines and unfinished reservations deliberately survive the new period.
}
pub fn observe(s: &mut Value, owner: &str, repo: &str, total: Option<u64>, ended: bool) -> Res<()> {
    if owner.is_empty() { crate::fail!("A shared budget claim needs an owner"); }
    if !s.at("owners").at(owner).is_object() {
        // First observation is the enrollment baseline, not a retrospective bill for earlier tasks.
        s["owners"][owner] = json!({ "repo": repo, "last": total.unwrap_or(0), "held": 0, "active": false, "uncertain": total.is_none() });
    }
    let old = s.at("owners").at(owner).clone();
    if old.s("repo") != repo { crate::fail!("A shared budget owner cannot move spending to another project"); }
    let total = total.map(|n| n.max(numeric_floor(&old))).or_else(||
        (ended && !old.b("active") && old.at("manualFloor").is_number() && !old.b("uncertain")).then(|| n(&old, "last")));
    if let Some(total) = total {
        if total < n(&old, "last") { s["owners"][owner]["uncertain"] = json!(true); }
        else {
            let delta = total - n(&old, "last");
            s["used"] = json!(n(s, "used").saturating_add(delta));
            if !repo.is_empty() { s["projects"][repo] = json!(s.at("projects").at(repo).as_u64().unwrap_or(0).saturating_add(delta)); }
            s["owners"][owner]["last"] = json!(total);
            s["owners"][owner]["held"] = json!(n(&old, "held").saturating_sub(delta));
            s["owners"][owner]["uncertain"] = json!(false);
        }
    } else { s["owners"][owner]["uncertain"] = json!(true); }
    s["owners"][owner]["observedAt"] = json!(crate::clock::now());
    if ended {
        s["owners"][owner]["active"] = json!(false);
        if !s.at("owners").at(owner).b("uncertain") { s["owners"][owner]["held"] = json!(0); }
    }
    // Completed one-shot helpers have no later callback. Keep recent details and aggregate all spending above.
    let mut completed: Vec<(String, i64)> = s.at("owners").as_object().unwrap().iter()
        .filter(|(id, o)| id.starts_with("helper:") && !o.b("active") && !o.b("uncertain"))
        .map(|(id, o)| (id.clone(), o.i("observedAt"))).collect();
    completed.sort_by_key(|(_, at)| *at);
    let remove = completed.len().saturating_sub(256);
    for (id, _) in completed.into_iter().take(remove) { s["owners"].as_object_mut().unwrap().shift_remove(&id); }
    Ok(())
}
fn numeric_floor(o: &Value) -> u64 { o.at("manualFloor").as_u64().unwrap_or(0) }
fn reserved(s: &Value, repo: Option<&str>) -> u64 {
    s.at("owners").as_object().unwrap().values().filter(|o| repo.is_none_or(|r| o.s("repo") == r))
        .fold(0u64, |a, o| a.saturating_add(n(o, "held")))
}
fn free(limit: Option<u64>, used: u64, held: u64, review: bool) -> u64 {
    let Some(limit) = limit else { return u64::MAX };
    let floor = if review { 0 } else { (limit as u128 * config::review_percent() as u128 / 100) as u64 };
    limit.saturating_sub(used.saturating_add(held).saturating_add(floor))
}
pub fn acquire(s: &mut Value, owner: &str, repo: &str, role: &str) -> Res<Value> {
    let review = role == "reviewer";
    let uncertain = s.at("owners").as_object().unwrap().iter().any(|(id, o)| o.b("uncertain") &&
        (id == owner || config::global().is_some() || (!repo.is_empty() && o.s("repo") == repo)));
    if config::enabled(repo) && uncertain { crate::fail!("Shared budget has unknown usage; inspect `herdr-board budget` and reconcile it before starting more model work"); }
    let old = s.at("owners").at(owner).clone();
    let held = if config::enabled(repo) {
        let own = n(&old, "held");
        let a = free(config::global(), n(s, "used"), reserved(s, None).saturating_sub(own), review);
        let b = free(config::project(repo), s.at("projects").at(repo).as_u64().unwrap_or(0), reserved(s, Some(repo)).saturating_sub(own), review);
        (if old.b("active") && own > 0 { own } else { config::quantum() }).min(a).min(b)
    } else { 0 };
    if config::enabled(repo) && held == 0 { crate::fail!("Shared token budget has no available allowance for {role}; wait for its next period or ask the human to raise it"); }
    s["owners"][owner]["held"] = json!(held);
    if !old.b("active") { s["owners"][owner]["startedAt"] = json!(crate::clock::now()); }
    s["owners"][owner]["active"] = json!(true);
    s["owners"][owner]["role"] = json!(role);
    Ok(json!({ "owner": owner, "reserved": held, "period": s.at("window"), "reviewPercent": config::review_percent() }))
}
pub fn status(s: &Value) -> Value {
    let projects: Obj = crate::store::settings::places::yours().repos.keys().chain(s.at("projects").as_object().unwrap().keys()).map(|repo| {
        (repo.clone(), json!({ "limit": config::project(repo), "used": s.at("projects").at(repo).as_u64().unwrap_or(0), "reserved": reserved(s, Some(repo)) }))
    }).collect();
    json!({ "enabled": config::any_enabled(), "window": s.at("window"), "global": { "limit": config::global(), "used": s.at("used"), "reserved": reserved(s, None) },
        "projects": projects, "owners": s.at("owners"), "reviewPercent": config::review_percent(), "reserve": config::quantum(),
        "scope": "this machine and this board home; observed tokens include cache; in-flight calls can overshoot" })
}
