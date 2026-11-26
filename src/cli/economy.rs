//! Status, cached delivery summaries and measured usage: compact read-only views, without a model turn.
use super::flags::Args;
use crate::err::Res;
use crate::json::*;
use crate::task_status::{said, status, tasks};
pub fn command(cmd: &str, a: &Args) -> Res<(Value, String)> {
    if a.flags.has("repo") && a.flags.s("repo").is_empty() { crate::fail!("--repo needs a repo name"); }
    let ids: Vec<String> = a.pos.iter().skip(1).cloned().collect();
    if cmd == "status" { let rows = status(&ids, a.flags.b("all"), a.flags.s("repo"))?; let words = said(&rows); return Ok((Value::Array(rows), words)); }
    let mut rows = vec![];
    for mut t in tasks(&ids, a.flags.b("all"), a.flags.s("repo"))? {
        rows.push(if cmd == "summary" { json!({ "id": t.at("id"), "summary": crate::digest::cached(&t), "evidence": crate::digest::evidence::references(&t), "generation": t.at("digestAttempt"), "current": crate::task_status::snapshot(&t) }) } else {
            if let Some(c) = crate::limits::spent::count(t.s("id"), "agent", 300) { t = c; }
            json!({ "id": t.at("id"), "total": t.at("spent").at("total"), "metrics": t.at("spent").at("metrics"), "partial": t.at("spent").at("partial"),
                "sessions": t.at("spent").at("sessions"), "subagents": t.at("spent").at("subagents"), "pendingSessions": t.at("spent").at("owed"),
                "summarizer": t.at("digestUsage"), "monetaryCost": null })
        });
    }
    let words = if rows.is_empty() { "No tasks.".into() } else { rows.iter().map(|v| serde_json::to_string(v).unwrap_or_default()).collect::<Vec<_>>().join("\n") };
    Ok((Value::Array(rows), words))
}
