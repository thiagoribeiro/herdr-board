//! Shared local token budgets: durable reservations, observed spending and explicit reconciliation of unknown outcomes.
pub mod config;
pub mod evidence;
pub mod ledger;
pub mod view;
use crate::{err::Res, json::*, store::files::{data_dir, read_json, with_lock_wait, write_json}};
pub fn exists() -> bool { data_dir().join("budget-ledger.json").exists() }
pub fn tracking() -> bool { exists() || config::any_enabled() }

/// Every mutation is serialized across board processes. Invalid state is never replaced by a fresh allowance.
fn change<T>(f: impl FnOnce(&mut Value) -> Res<T>) -> Res<T> {
    config::check()?;
    with_lock_wait("shared-budget", 5000, || {
        let path = data_dir().join("budget-ledger.json");
        let mut state = if path.exists() { read_json(&path).ok_or("Shared budget ledger is unreadable; inspect it before starting more work")? } else { ledger::new() };
        ledger::validate(&state)?;
        ledger::window(&mut state, crate::clock::now());
        let result = f(&mut state);
        write_json(&path, &state)?;
        result
    })
}

/// Claim a slice before starting or continuing model work. Repeating it for an active owner reuses that slice.
pub fn acquire(owner: &str, repo: &str, role: &str, total: Option<u64>) -> Res<Value> {
    acquire_measured(owner, repo, role, total, &Value::Null)
}
pub fn acquire_measured(owner: &str, repo: &str, role: &str, total: Option<u64>, files: &Value) -> Res<Value> {
    change(|s| { evidence::observe(s, owner, repo, total, false, files)?; ledger::acquire(s, owner, repo, role) })
}
/// Final evidence releases a reservation; unknown outcomes keep it held even after a process or turn ends.
pub fn observe(owner: &str, repo: &str, total: Option<u64>, ended: bool) -> Res<()> {
    observe_measured(owner, repo, total, ended, &Value::Null)
}
pub fn observe_measured(owner: &str, repo: &str, total: Option<u64>, ended: bool, files: &Value) -> Res<()> {
    if !tracking() { return Ok(()); }
    change(|s| evidence::observe(s, owner, repo, total, ended, files))
}
pub fn status() -> Res<Value> {
    if !tracking() { return Ok(json!({ "enabled": false, "owners": {}, "global": { "limit": null, "used": 0, "reserved": 0 } })); }
    change(|s| Ok(ledger::status(s)))
}
/// Only an explicit human reconciliation may resolve usage that no provider transcript could supply.
pub fn settle(owner: &str, total: u64) -> Res<()> {
    settle_confirmed(owner, total, false)
}
/// A human may certify an orphaned call's end after checking the provider, even if its callback was lost.
pub fn settle_confirmed(owner: &str, total: u64, ended: bool) -> Res<()> {
    change(|s| {
        let o = s.at("owners").at(owner).clone();
        if !o.is_object() { crate::fail!("No shared-budget owner {owner}"); }
        if o.b("active") && !ended { crate::fail!("Stop {owner} before reconciling its usage; a lost callback needs an explicit --ended after verifying the call ended"); }
        if total < o.at("last").as_u64().unwrap_or(0) { crate::fail!("Reconciled usage cannot erase observed spending"); }
        ledger::observe(s, owner, o.s("repo"), Some(total), true)?;
        s["owners"][owner]["reconciledAt"] = json!(crate::clock::now());
        s["owners"][owner]["manualFloor"] = json!(total);
        s["owners"][owner]["humanConfirmedEnd"] = json!(ended);
        Ok(())
    })
}
pub fn tokens(v: &Value) -> Option<u64> { v.as_f64().filter(|n| n.is_finite() && *n >= 0.0 && *n < u64::MAX as f64).map(|n| n.ceil() as u64) }
