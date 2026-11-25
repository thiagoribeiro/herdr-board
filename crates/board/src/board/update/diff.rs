//! The key that shows a task's change in your own diff tool (`board.diffTool`): a herdr pane split off beside the board,
//! in the task's folder, that runs the tool and closes when it ends, so quitting it is back on the board.
use super::Effect;
use crate::board::job::{Done, Job, Then};
use crate::board::state::State;
use crate::herdr::Herdr;
use crate::json::*;
use crate::shown::ink::Ink;
use crate::store::repos::{cwd_of, git_out};

/// The key: only a task of this machine, from a board in herdr.
pub fn key(s: &mut State, r: &Obj, fx: &mut Vec<Effect>) {
    let beside = if s.ctx.pane.is_empty() { s.ctx.from.clone() } else { s.ctx.pane.clone() };
    if s.ctx.web || r.b("host") || beside.is_empty() { return s.flash("A task's change opens in a herdr pane beside the board, on the machine it runs on: open the board there, in herdr.", Ink::Muted); }
    fx.push(Effect::Run(Job::Diff { id: r.t("id"), beside, popup: s.ctx.popup }));
}
/// The command the pane runs: `board.diffTool` with `{from}` the commit you last approved when the branch moved on since,
/// else where the task left main. None: its folder has no commit to compare.
pub fn command(t: &Obj) -> Option<String> {
    let dir = cwd_of(t);
    let head = git_out(&dir, &["rev-parse", "HEAD"]).ok()?;
    let approved = t.s("approvedHead");
    let from = if !approved.is_empty() && approved != head { approved.to_string() } else {
        let base = crate::plan::untouched::base_of(t);
        git_out(&dir, &["merge-base", "HEAD", if base.is_empty() { "HEAD" } else { &base }]).ok()?
    };
    Some(crate::store::settings::lookup::setting("board.diffTool", None).as_str().unwrap_or_default().replace("{from}", &from))
}
/// The job: the pane, the tool typed at its shell with `exit` after it, and the keyboard on it.
pub fn open_pane(id: &str, beside: &str, popup: bool) -> Done {
    let Some(t) = crate::store::tasks::get(id) else { return Done::fail(format!("{id} is gone.")) };
    let Some(cmd) = command(&t) else { return Done::say(format!("{id} has no git history to compare."), Ink::Yellow) };
    let api = Herdr::new(None);
    let run = || -> Result<(), crate::herdr::HerdrError> {
        let pane = api.pane_split(beside, &cwd_of(&t).display().to_string(), "right", &[])?.t("pane_id");
        api.pane_run(&pane, &format!("{cmd}; exit"))?;
        api.pane_focus(&pane)
    };
    if let Err(e) = run() { return Done::fail(format!("herdr did not open the diff pane: {}", if e.message.is_empty() { e.code } else { e.message })); }
    if popup { Done::default().and(Then::Leave) } else { Done::default() }
}
