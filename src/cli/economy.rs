//! Status, cached delivery summaries and measured usage: compact read-only views, without a model turn.
use super::flags::Args;
use crate::err::Res;
use crate::json::*;
use crate::store::tasks::{find, list, ListOpts};
pub fn command(cmd: &str, a: &Args) -> Res<(Value, String)> {
    if a.flags.has("repo") && a.flags.s("repo").is_empty() { crate::fail!("--repo needs a repo name"); }
    let ids: Vec<String> = a.pos.iter().skip(1).flat_map(|s| s.split(',')).map(String::from).collect();
    let tasks = if ids.is_empty() { list(ListOpts { include_archived: a.flags.b("all"), ..Default::default() }) } else {
        ids.iter().map(|id| find(id)).collect::<Res<Vec<_>>>()?
    };
    let mut rows = vec![];
    let live = if cmd == "status" { crate::flow::live::board(true) } else { vec![] };
    for mut t in tasks {
        if a.flags.b("repo") && t.s("repo") != a.flags.s("repo") { continue; }
        if ids.is_empty() && cmd == "status" && !a.flags.b("all") && (t.s("stage") == "done" || t.b("archivedAt")) { continue; }
        let v = match cmd {
            "summary" => json!({ "id": t.at("id"), "summary": crate::digest::cached(&t), "evidence": crate::digest::evidence::references(&t), "generation": t.at("digestAttempt"), "current": crate::task_status::snapshot(&t) }),
            "usage" => {
                if let Some(c) = crate::limits::spent::count(t.s("id"), "agent", 300) { t = c; }
                json!({ "id": t.at("id"), "total": t.at("spent").at("total"), "metrics": t.at("spent").at("metrics"), "partial": t.at("spent").at("partial"), "summarizer": t.at("digestUsage"), "monetaryCost": null })
            }
            _ => crate::task_status::snapshot(live.iter().find(|r| r.s("id") == t.s("id")).unwrap_or(&t)),
        };
        rows.push(v);
    }
    let words = if rows.is_empty() { "No tasks.".into() } else { rows.iter().map(|v| {
        if cmd == "status" { crate::task_status::line(v) } else { serde_json::to_string(v).unwrap_or_default() }
    }).collect::<Vec<_>>().join("\n") };
    Ok((Value::Array(rows), words))
}
