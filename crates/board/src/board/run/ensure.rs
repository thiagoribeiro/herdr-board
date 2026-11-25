//! The board opened when herdr starts (`board.openOnStartup`, docs/open-at-start.md): ensure-open, which unlike the
//! board key never closes a board, never goes back to where you came from and never opens a second one. It asks no
//! agent anything: herdr only.
use super::place::{go_to, open_tab, popup_wanted, running};
use crate::herdr::cli::{current_socket, session_name};
use crate::herdr::Herdr;
use crate::json::*;
use crate::store::{files::with_lock_wait, settings::lookup::setting};
use std::time::{Duration, Instant};

/// The title herdr gives the board's pane (`[[panes]]` in herdr-plugin.toml): how a board herdr restored, or one
/// another call just opened, is known before it has written where it runs.
pub const PANE_LABEL: &str = "Work board";

/// Make sure this herdr has a board: the one that runs, else a board pane herdr has that runs the board (one just opened
/// by a call racing this one: the look and the open are one step under a lock per herdr session), else a new one in a
/// tab. herdr (0.9.5) brings the board's pane back after a restart as a bare shell: no board, so a new one opens and
/// that shell, idle, is closed, the keyboard going to the new board where it was on the old one. `focus`: a new board
/// takes the keyboard; one that was there already is left as it is. Answers its pane, "" when herdr would not open one.
/// Nothing with the popup chosen: the popup is opened by the key only.
pub fn ensure(socket: &str, focus: bool) -> String {
    if popup_wanted() { return String::new(); }
    with_lock_wait(&format!("board-open-{}", session_name(socket)), 30000, || {
        if let Some((_, n, _)) = running(socket) { return Ok(n.t("pane")); }
        let api = Herdr::new(Some(socket.into()));
        let all = api.panes(None).unwrap_or_default();
        let panes: Vec<&Value> = all.iter().filter(|p| p.s("label") == PANE_LABEL).collect();
        if let Some(p) = panes.iter().find(|p| runs_board(&api, p.s("pane_id"))) { return Ok(p.t("pane_id")); }
        let Some(pane) = open_tab(&api) else { return Ok(String::new()) };
        let left: Vec<&Value> = panes.into_iter().filter(|p| api.pane_idle(p.s("pane_id"))).collect();
        // herdr gives the new tab the keyboard: it goes back to the pane that had it, unless the board is to have it.
        let to = if focus || left.iter().any(|p| p.b("focused")) { Some(&pane) } else { all.iter().find(|p| p.b("focused")) };
        if let Some(p) = to { go_to(&api, p); }
        for p in left { api.pane_close(p.s("pane_id")); }
        Ok(pane.t("pane_id"))
    }).unwrap_or_default()
}
/// Does the board run in this pane: its program, or bin/run.sh still starting it? Unknown: yes, so none opens twice.
fn runs_board(api: &Herdr, pane: &str) -> bool {
    api.process_info(pane).map(|i| i.arr("foreground_processes").iter().any(|f| f.s("name").starts_with("herdr-board") || f.s("cmdline").contains("run.sh"))).unwrap_or(true)
}

/// `herdr-board ensure-open`, which herdr's startup hook starts detached so herdr's start waits for nothing: herdr's
/// socket and its first space are waited for (a minute at most: a server no client has attached to has none), and a
/// board herdr restores is given a moment to come up. The keyboard goes to a new board only with
/// `board.focusOnStartup`, when herdr answered at once (a fresh start) and nobody moved it meanwhile.
pub fn at_startup() -> i32 {
    let socket = current_socket();
    let api = Herdr::new(Some(socket.clone()));
    let start = Instant::now();
    while api.workspaces().map(|w| w.is_empty()).unwrap_or(true) { // herdr opens no tab before it has a space
        if start.elapsed() > Duration::from_secs(60) { return 1; }
        std::thread::sleep(Duration::from_millis(250));
    }
    let focused = || api.panes(None).unwrap_or_default().into_iter().find(|p| p.b("focused")).map(|p| p.t("pane_id"));
    let before = focused();
    std::thread::sleep(Duration::from_millis(1500));
    let calm = start.elapsed() < Duration::from_secs(10) && focused() == before;
    ensure(&socket, calm && setting("board.focusOnStartup", None) == json!(true));
    0
}
