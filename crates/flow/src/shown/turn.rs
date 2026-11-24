//! Whose turn a task is and its next step, for `herdr-board status` (docs/status.md): herdr's sidebar's own answer
//! (states.rs: `who_stands`), with what only a turn says on top: a stalled task is yours to look at, a merge that waits
//! for its checks is GitHub's, a retry the board does by itself is the board's.
use super::queue::queued;
use super::row_says::sentence;
use super::states::{as_row, in_review, standing, who_stands};
use super::words::{age, pr_number};
use crate::json::*;
use crate::status_words::who::{AGENT, BOARD, GITHUB, REVIEWER, YOU};
use crate::store::retries::waiting_of;

/// Whose move it is: `you`, `agent` (its reviewer too), `board` or `github`. `r`: a row (as_row).
pub fn whose(r: &Obj) -> &'static str {
    if r.at("stall").s("event") == "stalled" { return YOU; }
    if r.s("stage") == "merging" && r.b("checksWait") { return GITHUB; }
    if waiting_of(r).is_some() || r.at("stall").s("event") == "unreachable" || queued(r) { return BOARD; }
    match who_stands(r).0.as_str() { AGENT | REVIEWER => AGENT, BOARD => BOARD, _ => YOU }
}
/// The next step, in one sentence: "delegate it", "approve PR#64 · Pass · 1 minor", "No change for 14h · Finishing".
pub fn next_step(r: &Obj) -> String {
    if r.s("stage") == "todo" { return "delegate it".into(); }
    let said = sentence(r).text();
    if in_review(r) && !r.b("stall") {
        let what = if r.b("pr") { pr_number(r) } else { "the merge".into() };
        return [format!("approve {what}"), said].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · ");
    }
    if said.is_empty() { standing(r) } else { said }
}
/// One task's line of `status`, as data: `t` is a task as the board shows it.
pub fn status_of(t: &Obj, now: i64) -> Value {
    let r = as_row(t);
    json!({ "id": r.at("id"), "title": r.at("title"), "stage": r.at("stage"), "state": standing(&r), "since": r.at("at"), "for": age(now - r.i("at")),
        "whose": whose(&r), "next": next_step(&r), "stall": r.at("stall") })
}
