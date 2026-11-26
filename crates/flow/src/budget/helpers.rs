//! Measured, tool-free helper claims: unknown results retain their reservation instead of becoming zero usage.
use crate::{err::Res, json::*};
pub struct Claim { owner: String, repo: String }
pub fn begin(label: &str, repo: &str, kind: &str) -> Res<Option<Claim>> {
    if !super::config::enabled(repo) { return Ok(None); }
    if kind != "claude" { crate::fail!("Shared budgets defer {label}: {kind} has no measured one-shot envelope"); }
    let owner = format!("helper:{label}:{}", crate::util::uuid());
    super::acquire(&owner, repo, "helper", Some(0))?;
    Ok(Some(Claim { owner, repo: repo.into() }))
}
/// The returned text stays the program's result; accounting never changes task evidence or grants an approval.
pub fn finish(claim: Option<Claim>, out: &str) -> String {
    let Some(claim) = claim else { return out.into() };
    let envelope = serde_json::from_str::<Value>(out).ok();
    let measured = envelope.as_ref().filter(|e| e.at("usage").is_object() && e.at("result").is_string());
    let used = measured.and_then(|e| {
        let u = e.at("usage");
        if !u.at("input_tokens").is_number() || !u.at("output_tokens").is_number() { return None; }
        if ["input_tokens", "cache_creation_input_tokens", "cache_read_input_tokens", "output_tokens"].iter().any(|k| u.get(*k).is_some_and(|v| super::tokens(v).is_none())) { return None; }
        super::tokens(&json!(["input_tokens", "cache_creation_input_tokens", "cache_read_input_tokens", "output_tokens"].iter().map(|k| u.f(k)).sum::<f64>()))
    });
    let _ = super::observe(&claim.owner, &claim.repo, used, true);
    measured.map(|e| e.s("result").to_string()).unwrap_or_else(|| out.into())
}
