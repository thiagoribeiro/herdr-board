//! Provider evidence identities shared across owners, including adopted attempts and copied Claude replies.
use crate::{err::Res, json::*};

fn sum(v: &Value) -> f64 { ["input", "cacheWrite", "cacheRead", "output"].iter().map(|k| v.f(k)).sum() }
fn sources(files: &Value) -> Option<Obj> {
    let mut out = Obj::new();
    for (path, file) in files.as_object()? {
        let seen = file.o("seen");
        let accounted: f64 = seen.into_iter().flat_map(|m| m.values()).map(sum).sum();
        if file.s("kind") == "claude" && seen.is_some() && (accounted - file.f("n")).abs() < 0.5 {
            for (id, parts) in seen.unwrap() {
                let key = format!("claude:{id}");
                let n = super::tokens(&json!(sum(parts)))?;
                let old = out.at(&key).as_u64().unwrap_or(0);
                out.put(&key, old.max(n));
            }
        } else {
            let path = std::fs::canonicalize(path).map(|p| p.display().to_string()).unwrap_or_else(|_| path.clone());
            out.put(&format!("{}:{path}", file.s("kind")), super::tokens(file.at("n"))?);
        }
    }
    Some(out)
}

/// Owner counters remain useful for reconciliation; shared evidence determines the newly billable delta.
pub fn observe(s: &mut Value, owner: &str, repo: &str, total: Option<u64>, ended: bool, files: &Value) -> Res<()> {
    let evidence = total.and_then(|_| sources(files)).filter(|f| !f.is_empty());
    let Some(evidence) = evidence else { return super::ledger::observe(s, owner, repo, total, ended) };
    let before = s.at("owners").at(owner).clone();
    let known = before.is_object();
    let held = before.at("held").as_u64().unwrap_or(0);
    let last = before.at("last").as_u64().unwrap_or(0);
    let used = s.at("used").as_u64().unwrap_or(0);
    let project = s.at("projects").at(repo).as_u64().unwrap_or(0);
    super::ledger::observe(s, owner, repo, total, ended)?;
    if s.at("owners").at(owner).b("uncertain") { return Ok(()); }
    if !s.at("sources").is_object() { s["sources"] = json!({}); }
    let mut fresh = 0u64;
    for (key, amount) in evidence {
        let amount = amount.as_u64().unwrap();
        let prior = s.at("sources").at(&key).as_u64().unwrap_or(0);
        fresh = fresh.saturating_add(amount.saturating_sub(prior));
        s["sources"][&key] = json!(prior.max(amount));
    }
    // Enrollment establishes a baseline. A manual reconciliation may already have billed some unseen evidence.
    let delta = if known { fresh.min(total.unwrap().saturating_sub(last)) } else { 0 };
    s["used"] = json!(used.saturating_add(delta));
    if !repo.is_empty() { s["projects"][repo] = json!(project.saturating_add(delta)); }
    s["owners"][owner]["held"] = json!(if ended { 0 } else { held.saturating_sub(delta) });
    Ok(())
}
