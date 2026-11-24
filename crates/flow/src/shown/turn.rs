//! Whose turn a task is and its next step, for `herdr-board status` (docs/status.md): you, its agent, the board or
//! GitHub, and the next step in one sentence, from the row as the board shows it (flow/live.rs) and nothing else.
use super::row_says::sentence;
use super::states::{as_row, in_review, needs_you, standing, under_review};
use super::words::pr_number;
use crate::json::*;
use crate::store::retries::waiting_of;

pub const YOU: &str = "you";
pub const AGENT: &str = "agent";
pub const BOARD: &str = "board";
pub const GITHUB: &str = "github";
/// Whose move it is: the board's while it retries by itself, GitHub's while a merge waits for its checks.
pub fn whose(r: &Obj) -> &'static str {
    if r.s("stage") == "merging" && r.b("checksWait") { return GITHUB; }
    if waiting_of(r).is_some() || r.at("stall").s("event") == "unreachable" { return BOARD; }
    if needs_you(r) || r.s("stage") == "approval" || r.s("stage") == "progress" { return YOU; }
    if under_review(r) || r.s("stage") == "working" { return AGENT; }
    BOARD
}
/// The next step, in one sentence: "approve PR#64 · Pass · 1 minor", "Waiting for GitHub checks · 0 of 1 passed".
pub fn next_step(r: &Obj) -> String {
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
    json!({ "id": r.at("id"), "title": r.at("title"), "stage": r.at("stage"), "state": standing(&r), "since": r.at("at"), "for": super::words::age(now - r.i("at")),
        "whose": whose(&r), "next": next_step(&r), "stall": r.at("stall") })
}
