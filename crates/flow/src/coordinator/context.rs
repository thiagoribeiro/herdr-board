//! Per-message context captured at enqueue, carried inline and hidden from chat. Task results stay separate from live state.
use super::{co::Co, state::{get, change}};
use crate::json::*;
use crate::store::tasks::{find, list, ListOpts};
pub const OPEN: &str = "\n\n<herdr-board-context>\n";
pub const CLOSE: &str = "\n</herdr-board-context>";
/// Resolve only explicit identifiers and the selected task. Never guess a task from a common word in its title.
pub fn relevant(text: &str, selected: &str) -> Vec<Obj> {
    let mut ids = vec![];
    if !selected.is_empty() { ids.push(selected.to_string()); }
    for word in text.split_whitespace().map(|s| s.trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '/' && c != ':')) {
        if (word.contains('/') || word.contains('-')) && word.chars().last().is_some_and(|c| c.is_ascii_digit()) {
            if let Ok(t) = find(word) { if !ids.contains(&t.t("id")) { ids.push(t.t("id")); } }
        }
    }
    ids.iter().filter_map(|id| find(id).ok()).collect()
}
pub fn capture(where_: &Value, text: &str, sender: &Value) -> Value {
    json!({ "capturedAt": crate::clock::now(), "repo": where_.at("repo"), "root": where_.at("root"),
        "selectedTask": where_.at("task"), "delegate": where_.at("delegate"), "sender": sender,
        "tasks": relevant(text, where_.s("task")).iter().map(crate::task_status::snapshot).collect::<Vec<_>>() })
}
pub fn wrap(co: &Co, message: &Value, words: &str) -> String {
    use crate::store::settings::{coord::Coord, lookup::{lookup, Scope}, places::yours};
    if lookup("coordinator.context", &Scope::coord(Coord::of(&co.settings_key())), &yours()).is_ok_and(|f| f.value == json!(false)) { return words.into(); }
    let mut ctx = if message.at("context").is_object() { message.at("context").clone() } else { capture(message.at("where"), message.s("said"), message.at("sender")) };
    ctx["requestId"] = message.at("id").clone();
    if let Some(c) = get(co) {
        if c.at("memory").is_object() { ctx["memory"] = c.at("memory").clone(); }
        ctx["contextTokens"] = c.at("contextTokens").clone();
        let budget = lookup("coordinator.contextBudget", &Scope::coord(Coord::of(&co.settings_key())), &yours()).ok().map(|f| crate::store::settings::values::tokens_of(&text(&f.value))).unwrap_or(0);
        ctx["checkpointSuggested"] = json!(budget > 0 && c.f("contextTokens") > budget as f64);
    }
    if ctx.to_string().len() > 48000 {
        if let Some(tasks) = ctx["tasks"].as_array_mut() { for task in tasks { if task.b("summary") { task["summaryAvailable"] = json!(true); task.as_object_mut().unwrap().remove("summary"); } } }
        ctx["detailsAvailable"] = json!("Use status/summary for selected ids; original requirements are in details");
    }
    format!("{words}{OPEN}{ctx}{CLOSE}")
}
pub fn bare(s: &str) -> String {
    if s.ends_with(CLOSE) { if let Some(i) = s.rfind(OPEN) { return s[..i].to_string(); } }
    s.into()
}
/// Durable decisions supplied explicitly by the coordinator, not inferred by a helper. Originals and sessions remain intact.
pub fn remember(co: &Co, v: &Value) -> crate::err::Res<Obj> {
    if !v.is_object() || v.to_string().len() > 12000 { crate::fail!("Memory must be a JSON object of at most 12 KB; keep decisions, constraints and open questions, not logs"); }
    for k in ["decisions", "constraints", "questions"] {
        if !v.at(k).is_array() || v.arr(k).iter().any(|x| !x.is_string()) { crate::fail!("Memory needs string arrays: decisions, constraints, questions"); }
    }
    change(co, |c| { c.put("memory", json!({ "at": crate::clock::now(), "content": v })); true })
}
/// A compact fresh view for questions about a project; never starts a task or an agent.
pub fn project(repo: &str) -> Value {
    Value::Array(list(ListOpts::default()).iter().filter(|t| t.s("repo") == repo && t.s("stage") != "done").map(crate::task_status::snapshot).collect())
}
