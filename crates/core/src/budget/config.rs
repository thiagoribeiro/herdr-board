//! Budget settings resolved from the human's machine and project entries; task code cannot supply them.
use crate::{json::*, store::settings::{lookup::{lookup, Scope}, places::yours, values::{ms, tokens_of}}};
fn value(name: &str, repo: &str) -> Value { lookup(name, &Scope::repo(repo), &yours()).map(|f| f.value).unwrap_or(Value::Null) }
fn count(name: &str, repo: &str) -> Option<u64> { let n = tokens_of(&text(&value(name, repo))); (n > 0).then_some(n as u64) }
pub fn global() -> Option<u64> { count("budget.tokens", "") }
pub fn project(repo: &str) -> Option<u64> { if repo.is_empty() { None } else { count("budget.projectTokens", repo) } }
pub fn invalid() -> bool { yours().problems.iter().any(|p| p.name.starts_with("budget.") || p.name.is_empty()) }
pub fn check() -> crate::err::Res<()> {
    if invalid() { crate::fail!("Shared budget settings are invalid; fix the human's settings before starting model work"); }
    Ok(())
}
pub fn enabled(repo: &str) -> bool { invalid() || global().is_some() || project(repo).is_some() }
pub fn any_enabled() -> bool { invalid() || global().is_some() || yours().repos.keys().any(|r| project(r).is_some()) }
pub fn period() -> i64 { ms(&text(&value("budget.period", ""))).max(1000) }
pub fn quantum() -> u64 { count("budget.reserve", "").unwrap_or(100000).max(1) }
pub fn review_percent() -> u64 { value("budget.reviewPercent", "").as_u64().unwrap_or(20).min(100) }
