//! Compact budget snapshots for task panels and coordinator prompts, without loading the owners' full ledger into context.
use crate::json::*;
pub fn snapshot(owner: &str, repo: &str) -> Value {
    if !super::config::enabled(repo) { return Value::Null; }
    match super::status() {
        Ok(s) => json!({ "observedAt": crate::clock::now(), "scope": "this machine", "window": s.at("window"),
            "global": s.at("global"), "project": s.at("projects").at(repo), "owner": s.at("owners").at(owner),
            "reviewPercent": s.at("reviewPercent"), "note": "Reservations are shared. In-flight calls can overshoot; status is evidence, not permission to raise caps." }),
        Err(e) => json!({ "observedAt": crate::clock::now(), "error": e.msg }),
    }
}
