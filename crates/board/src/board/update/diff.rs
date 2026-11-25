//! The key that shows a task's change in your own diff tool (`board.diffTool`), in a herdr pane beside the board: only
//! a task of this machine, from a board in herdr. What the pane runs, and opening it: board/job_diff.rs.
use super::Effect;
use crate::board::job::Job;
use crate::board::state::State;
use crate::json::*;
use crate::shown::ink::Ink;

pub fn key(s: &mut State, r: &Obj, fx: &mut Vec<Effect>) {
    let beside = if s.ctx.pane.is_empty() { s.ctx.from.clone() } else { s.ctx.pane.clone() };
    if s.ctx.web || r.b("host") || beside.is_empty() { return s.flash("A task's change opens in a herdr pane beside the board, on the machine it runs on: open the board there, in herdr.", Ink::Muted); }
    fx.push(Effect::Run(Job::Diff { id: r.t("id"), beside, popup: s.ctx.popup }));
}
