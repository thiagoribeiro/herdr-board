//! The key to the next task that needs you: it selects that task and puts the reply box on it, so what you type next
//! is your answer. In the order the list shows the tasks, round again from the row you are on; only the rows in view,
//! so a filter or a folded heading keeps what it hides. `board.nextAfterReply` moves on by itself after an answer.
use super::list::relist;
use crate::board::state::State;
use crate::json::*;
use crate::shown::ink::Ink;
use crate::shown::states::group_of;
use crate::status_words::words::NEEDS;

/// A task that waits for you: under Needs you, and not archived.
pub fn waits(r: &Obj) -> bool { !r.b("archivedAt") && group_of(r) == NEEDS }
/// Talk to the next task in view that needs you: the selected one when the chat is not on it yet, else the one after
/// it (the first, round again).
pub fn next(s: &mut State) {
    let here = s.steps.iter().position(|id| *id == s.sel);
    let on = s.reply && s.reply_id == s.sel;
    let need: Vec<(usize, &String)> = s.steps.iter().enumerate().filter(|(_, id)| s.items.iter().any(|r| r.s("id") == id.as_str() && waits(r))).collect();
    let Some(to) = need.iter().find(|(n, _)| Some(*n) > here || (!on && Some(*n) == here)).or(need.first()).map(|(_, id)| (*id).clone()) else { return s.flash("Nothing in view needs you.", Ink::Muted) };
    s.sel = to.clone(); s.reply = true; s.reply_id = to; s.panel_top = 0;
    relist(s);
}
/// After an answer sent to a task that waited for you: on to the next one, when `board.nextAfterReply` says so.
pub fn after_reply(s: &mut State, r: &Obj) { if s.data.next_after_reply && waits(r) { next(s); } }
