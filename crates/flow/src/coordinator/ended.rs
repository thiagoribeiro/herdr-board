//! A task's run ended (ready to approve, done, failed, stopped at a limit): the coordinator that added it (the
//! Coordinator for one you added) hears it in its chat, once per run, with the task's closing note, pull request and
//! review in the words a follow-up is handed (after/handed.rs). The message starts with `Board:`, and its rules have it
//! tell the human there what the task did (docs/coordinator.md). Only a task added since the board does this (`news`)
//! and a coordinator that was ever talked to hear it: nothing older is told all at once, and a coordinator that was stopped keeps it waiting.
use super::co::Co;
use super::state::{change, get};
use crate::clock::now;
use crate::herdr::cli::same_socket;
use crate::json::*;
use crate::store::tasks::{update, Change, Task};

/// The words a board message starts with.
pub const WORDS: &str = "Board:";
/// A message in a coordinator's queue that is the board's (an autopilot wake, a task that ended), not the human's words.
pub fn by_board(m: &Value) -> bool { m.b("autopilot") || m.b("board") }
/// A message of the board's, as its chat shows it: under whose name (`Autopilot`, `Board`), and what of it (why it
/// woke; the first line of a task that ended). None: the human's words.
pub fn shown_as(text: &str) -> Option<(&'static str, String)> {
    if crate::autopilot::wake::is_wake(text) { return Some(("Autopilot", crate::autopilot::wake::said(text))); }
    text.strip_prefix(WORDS).map(|t| ("Board", t.lines().next().unwrap_or("").trim().to_string()))
}
/// Words from anyone but the board that would pass for its message: marked as none.
pub fn escape(text: &str) -> String {
    let t = crate::autopilot::wake::escape(text);
    if t.trim_start().starts_with(WORDS) { format!("(not the board's) {}", t.trim_start()) } else { t }
}
/// How its run ended, in words: a run whose end was told once is not told again when only the stage moves on (approved
/// and merged), but a new run is. None while it has not ended.
fn end_of(t: &Obj) -> Option<(&'static str, &'static str)> {
    match t.s("stage") {
        "approval" => Some(("finished", "is ready to approve")),
        "done" => Some(("finished", "is done")),
        "failed" | "nosummary" => Some(("failed", "failed")),
        "stopped" if t.b("limitStop") => Some(("limit", "stopped at its limit")),
        _ => None,
    }
}
/// What is told of this run, if it is due: its mark (kept on the task as `told`) and its words.
fn due(t: &Obj) -> Option<(String, &'static str)> {
    if !t.b("news") || t.b("archivedAt") || t.b("attemptOf") { return None; }
    let (kind, words) = end_of(t)?;
    let mark = format!("{kind}:{}:{}", t.i("armedAt"), t.i("failedAt")); // a retry that fails again is a new failure
    (t.s("told") != mark).then_some((mark, words))
}
/// The coordinator it goes to: the one that added the task, the Coordinator for one you added.
fn co_of(t: &Obj, socket: &str) -> Co {
    let key = if t.b("coordinator") { t.s("coordinator") } else { t.at("autopilot").s("coordinator") };
    crate::autopilot::record::co_of(socket, key)
}
/// The message: why, and the task as a follow-up is handed it.
pub fn text_of(id: &str, words: &str) -> String {
    format!("{WORDS} {} {words}.\n\n{}", crate::after::say(id), crate::after::handed::line(&crate::after::handed::of(id)))
}
/// The housekeeping pass: each ended run of this herdr session's tasks into its coordinator's queue, once. A stopped
/// coordinator keeps it waiting until it is talked to again.
pub fn ended_pass(tasks: &[Task], socket: &str) {
    for seen in tasks {
        if due(seen).is_none() || !same_socket(seen.at("link").s("socket"), socket) { continue; }
        let co = co_of(seen, socket);
        if get(&co).is_none() { continue; }
        let mut words = "";
        let _ = update(seen.s("id"), |t| {
            let Some((mark, w)) = due(t) else { return Ok(Change::Keep) };
            t.put("told", mark); words = w;
            Ok(Change::Write)
        });
        if words.is_empty() { continue; }
        let text = text_of(seen.s("id"), words);
        let _ = change(&co, |c| {
            super::state::enqueue(c, json!({ "id": crate::util::uuid(), "text": text, "said": text, "board": true, "at": now() }));
            true
        });
    }
}
