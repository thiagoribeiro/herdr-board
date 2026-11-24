//! A task that stopped moving, found by fixed rules and never by a model: too long in the same stage (`stall.after`),
//! a pull request GitHub finds in conflict with main (`prConflict`, housekeeping/prs.rs), or main that could not be
//! fetched, with the time of the last try. The board's row, herdr's sidebar and `herdr-board status` say it, and the
//! first two are also `board.notify` events (notify.rs). docs/troubleshooting.md.
use crate::json::*;
use crate::shown::words::age;
use crate::store::settings::{lookup::setting, values::ms};
use crate::sync::main_word;

/// The stages a task may stand still in for too long: its agent's turn ("Finishing" too) and the board's merge.
const STALLS_IN: [&str; 2] = ["working", "merging"];
/// One stall: its event word (`stalled`, `conflict`, `unreachable`) and the reason, in plain words.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Stall { pub event: &'static str, pub why: String }
/// Since when nothing moved: its last stage change, or the last time herdr saw its agent work.
pub fn moved_at(t: &Obj) -> i64 { t.i("at").max(t.i("workedAt")) }
pub fn stall_of(t: &Obj, now: i64) -> Option<Stall> {
    if t.b("archivedAt") || ["done", "todo", "tried"].contains(&t.s("stage")) { return None; }
    let say = |event, why: String| Some(Stall { event, why });
    if t.b("prConflict") { return say("conflict", format!("Pull request conflicts with {}", main_word(t))); }
    let sb = t.at("syncBlocked");
    if sb.s("why").starts_with("main could not be fetched") {
        let at = chrono::DateTime::from_timestamp_millis(sb.i("at")).map(|d| d.with_timezone(&chrono::Local).format("%H:%M").to_string()).unwrap_or_default();
        return say("unreachable", format!("{} could not be fetched, last try {at}", main_word(t)));
    }
    if !STALLS_IN.contains(&t.s("stage")) || t.i("at") == 0 { return None; }
    let (after, still) = (ms(setting("stall.after", Some(t)).as_str().unwrap_or("")), now - moved_at(t));
    if after > 0 && still >= after { say("stalled", format!("No change for {}", age(still))) } else { None }
}
