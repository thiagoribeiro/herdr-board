//! What a task is told of the work before it: for each task it waited for (on this machine or another), or the one it
//! follows up on (`followUp`), its id, title, closing note and last review's findings, and the pull request and merge
//! commit where its work merged (its branch where it did not), or the report where it was research. What it waited for
//! is taken when the wait ends and kept on the task (`afterDone`), so a clean-up of the task before it loses nothing of
//! it. A coordinator hears of a task that ended in the same words (coordinator/news.rs).
use super::{after_of, say, stand_of, Stand};
use crate::clock::now;
use crate::flow::task::note_stat;
use crate::json::*;
use crate::research::is_research;
use crate::store::files::dir_of;
use crate::store::tasks::{get, Task};

/// The longest closing note handed over, in characters: a longer one is cut, and its task's `details` has the rest.
const NOTE_MAX: usize = 4000;
/// A task's closing note, wherever the board has it: the note itself, the copy the finish line kept, what it said last.
fn note_of(d: &Task) -> String {
    if let Some((_, n)) = note_stat(d) { if !n.is_empty() { return n; } }
    if let Ok(n) = std::fs::read_to_string(dir_of(d.s("id")).join("closing.md")) { if !n.trim().is_empty() { return n.trim().to_string(); } }
    [d.t("summary"), d.t("said")].into_iter().map(|s| s.trim().to_string()).find(|s| !s.is_empty()).unwrap_or_default()
}
/// One task waited for, as it is now.
pub fn of(id: &str) -> Value {
    if super::far::is_far(id) { return super::far::handed(id); } // a task of another machine: what it said last
    let Some(d) = get(id) else { return json!({ "id": id, "gone": true }) };
    let mut o = obj(json!({ "id": id, "title": d.at("title"), "repo": d.at("repo"), "done": stand_of(id) == Stand::Done, "stage": d.at("stage"), "research": is_research(&d) }));
    let note: String = note_of(&d).chars().take(NOTE_MAX).collect();
    if !note.is_empty() { o.put("note", note); }
    if d.b("pr") { o.put("pr", d.at("pr").clone()); }
    if d.b("branch") { o.put("branch", d.at("branch").clone()); }
    if let Some(rv) = crate::store::reviews::current_review(&d) { let f: String = crate::gate::findings_text(rv, true).chars().take(NOTE_MAX).collect(); if !f.trim().is_empty() { o.put("review", f); } }
    if d.b("prMerged") || d.s("prState") == "merged" || d.b("mergeCommit") { o.put("merged", true); }
    if d.b("mergeCommit") { o.put("commit", d.at("mergeCommit").clone()); }
    if d.at("report").b("link") { o.put("report", d.at("report").at("link").clone()); }
    Value::Object(o)
}
/// The wait is over (or cut short by you, `early`): the task goes on to a slot like any delegated one, with what it
/// waited for kept for its first prompt.
pub fn hand_over(t: &mut Task, early: bool) {
    let done: Vec<Value> = after_of(t).iter().map(|id| of(id)).collect();
    t.put("afterDone", done);
    if early { t.put("afterEarly", true); }
    for k in ["afterLeft", "afterBlocked"] { t.del(k); }
    t.put("stage", "queued"); t.put("queuedAt", now()); t.put("msg", "");
}
/// The part of the first prompt that hands over what came before: nothing for a task that waited for nothing and
/// follows up on nothing.
pub fn words(t: &Obj) -> String {
    let mut out: Vec<String> = vec![];
    if t.b("followUp") {
        out.push("This task follows up on the task below: it continues that work. Start from what it did (its change, its pull request, its closing note, its review), and keep to that scope unless this task's description says otherwise.".to_string());
        out.push(format!("- {}", line(&of(t.s("followUp")))));
    }
    let list = t.arr("afterDone");
    if list.is_empty() { return out.join("\n"); }
    if !out.is_empty() { out.push(String::new()); }
    out.push(if t.b("afterEarly") {
        "This task was to wait for the tasks below to be done, and the human started it before they all were. Take in what is there of their work, and leave alone what is not done yet.".to_string()
    } else { "This task waited for the tasks below to be done, and they are. It builds on their work: take it in (fetch it, read it, follow what it changed) before you go on.".to_string() });
    out.extend(list.iter().map(|d| format!("- {}", line(d))));
    out.join("\n")
}
/// One task as handed over (`of`): where it stands, its pull request or report, its closing note and its review.
pub fn line(d: &Value) -> String {
    let id = d.t("id");
    if d.b("gone") { return format!("{}: removed from the board, nothing is known of it.", say(&id)); }
    let on = if d.b("machine") { format!(" on {}", d.s("machine")) } else { String::new() };
    let mut line = format!("{} \"{}\" (repo {}{on}): {}", say(&id), d.s("title"), d.s("repo"), if d.b("done") { "done" } else { "not done yet" });
    if d.b("merged") {
        line.push_str(", its work merged");
        if d.b("pr") { line.push_str(&format!(": pull request {}", d.s("pr"))); }
        if d.b("commit") { line.push_str(&format!(", merge commit {}", d.s("commit"))); }
    } else if d.b("pr") { line.push_str(&format!(", pull request {} (not merged)", d.s("pr"))); }
    if d.b("branch") && !d.b("merged") { line.push_str(&format!(", its work on branch {}", d.s("branch"))); }
    if d.b("report") { line.push_str(&format!(", {}report: {}", if d.b("research") { "research only, its " } else { "its " }, d.s("report"))); }
    line.push('.');
    let indent = |s: &str| s.lines().map(|l| format!("    {l}")).collect::<Vec<_>>().join("\n");
    if d.b("note") { line.push_str(&format!(" Its closing note:\n{}", indent(d.s("note")))); }
    if d.b("review") { line.push_str(&format!("\n  Its last review:\n{}", indent(d.s("review")))); }
    line
}
