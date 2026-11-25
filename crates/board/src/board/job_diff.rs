//! The diff key's job: a herdr pane split off beside the board, in the task's folder, that runs your diff tool
//! (`board.diffTool`) with `exit` after it, so quitting the tool closes the pane and you are back on the board.
use super::job::{Done, Then};
use crate::herdr::Herdr;
use crate::json::*;
use crate::shown::ink::Ink;
use crate::store::repos::{cwd_of, git_out};

/// The command the pane runs: `board.diffTool` with `{from}` what the task's change is compared with. After an
/// approval that lapsed (`lapsedApproval`, finish/pipeline_end.rs): the commit you approved, with main merged into it as
/// the branch has it since (a tree, `git merge-tree`), so only the task's own change since your yes shows. Else, or
/// where that merge conflicts: where the task left main. None: its folder has no commit to compare.
pub fn command(t: &Obj) -> Option<String> {
    let dir = cwd_of(t);
    let git = |a: &[&str]| git_out(&dir, a).ok();
    let head = git(&["rev-parse", "HEAD"])?;
    let (approved, main) = (t.s("lapsedApproval"), t.s("syncedTo"));
    let since = if approved.is_empty() || approved == head { None }
        else if main.is_empty() || git(&["merge-base", "--is-ancestor", main, approved]).is_some() { Some(approved.to_string()) }
        else { git(&["merge-tree", "--write-tree", approved, main]).and_then(|o| o.lines().next().map(String::from)) };
    let from = match since { Some(f) => f, None => {
        let base = crate::plan::untouched::base_of(t);
        git(&["merge-base", "HEAD", if base.is_empty() { "HEAD" } else { &base }])?
    } };
    Some(crate::store::settings::lookup::setting("board.diffTool", None).as_str().unwrap_or_default().replace("{from}", &from))
}
/// The job: the pane, the tool typed at its shell with `exit` after it, and the keyboard on it. In the popup the pane is
/// split off the one you came from, and the popup closes.
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
    if popup { Done::default().and(Then::Leave("your diff tool")) } else { Done::default() }
}
