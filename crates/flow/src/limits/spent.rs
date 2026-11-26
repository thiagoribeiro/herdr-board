//! What a task has used: the tokens of every conversation it had (its agent's, each reviewer's, each new chat's), added
//! up on the task, including native subagents. Each conversation file has an incremental counter, retained under
//! `spent.files[<file>]`; shared Claude replies count once across the family. Missing conversations stay owed.
//! `spent.base` is where the count of
//! `limit.tokens` starts: 0, then the total at the moment you said to carry on after the limit stopped the task.
use crate::agents::{adapter, kinds::kind, token_tree};
use crate::json::*;
use crate::store::repos::cwd_of;
use crate::store::tasks::{get, update_wait, Change, Task};
use std::path::{Path, PathBuf};

/// Can this program's tokens be counted? Where not, `limit.tokens` does nothing for it.
pub fn can_count(k: &str) -> bool { kind(k).map(|a| a.tokens).unwrap_or(false) }
fn kind_of(t: &Task, role: &str) -> String { if role == "reviewer" { t.at("link").at("reviewer").t("kind") } else { t.t("agent") } }
/// A conversation the task had, as it is found again later: { kind, session, file, cwd }.
fn known(t: &Task, role: &str) -> Option<Value> {
    let k = kind_of(t, role);
    let session = if role == "reviewer" { t.t("reviewSession") } else { t.t("session") };
    let file = if role != "reviewer" && t.b("transcript") { t.t("transcript") } else { String::new() };
    (!k.is_empty() && (!session.is_empty() || !file.is_empty())).then(|| json!({ "kind": k, "session": session, "file": file, "cwd": cwd_of(t).display().to_string() }))
}
fn same(a: &Value, b: &Value) -> bool { a.s("kind") == b.s("kind") && a.s("session") == b.s("session") && a.s("file") == b.s("file") }
fn file_of(e: &Value) -> Option<PathBuf> {
    if !e.s("file").is_empty() && Path::new(e.s("file")).exists() { return Some(PathBuf::from(e.s("file"))); }
    if e.s("session").is_empty() { return None; }
    adapter::transcript(e.s("kind"), e.s("session"), Path::new(e.s("cwd")))
}
/// `spent` with every conversation the task is owed read first, then this role's read on; None when nothing changed.
pub fn counted(t: &Task, role: &str) -> Option<Obj> {
    let spent = t.o("spent").cloned().unwrap_or_default();
    let mut files = spent.o("files").cloned().unwrap_or_default();
    let mut changed = false;
    let mut owed = spent.arr("owed").to_vec();
    if let Some(e) = known(t, role) { if !owed.iter().any(|o| same(o, &e)) { owed.push(e); } }
    // Keep reading ended reviews and earlier workers: their children may save their last usage later.
    let roots: Vec<Value> = files.iter().filter(|(_, f)| f.b("kind") && !f.b("parent"))
        .map(|(path, f)| json!({ "kind": f.at("kind"), "session": f.at("session"), "file": path })).collect();
    let mut read_files = std::collections::HashSet::new();
    let mut read = |e: Option<&Value>| {
        let Some(e) = e else { return };
        if !can_count(e.s("kind")) { return; }
        let Some(file) = file_of(e) else {
            if let Some(counter) = files.get_mut(e.s("file")) {
                if !counter.b("unavailable") { counter["unavailable"] = json!(true); changed = true; }
            }
            return;
        };
        if !read_files.insert(file.clone()) { return; }
        changed |= token_tree::count(e.s("kind"), &file, e.s("session"), &mut files);
    };
    let unresolved: Vec<Value> = owed.iter().filter(|e| !can_count(e.s("kind")) || file_of(e).is_none()).cloned().collect();
    for e in roots.iter().chain(&owed) { read(Some(e)); }
    if !changed && unresolved == spent.arr("owed") { return None; }
    let mut rest = spent.clone();
    if unresolved.is_empty() { rest.del("owed"); } else { rest.put("owed", unresolved); }
    let summary = token_tree::totals(&files);
    rest.put("files", files);
    for k in ["total", "metrics", "sessions", "subagents"] { rest.put(k, summary.at(k).clone()); }
    rest.put("partial", summary.b("partial") || !rest.arr("owed").is_empty());
    Some(rest)
}
/// Count a role's conversation onto its task independently of limits. Missing or unsupported sessions remain owed.
pub fn count(id: &str, role: &str, wait_ms: i64) -> Option<Task> {
    let r = (|| -> crate::err::Res<Task> {
        let role = role.to_string();
        update_wait(id, wait_ms, move |t| { match counted(t, &role) { Some(s) => { t.put("spent", s); Ok(Change::Write) } None => Ok(Change::Keep) } })
    })();
    r.ok().or_else(|| get(id))
}
