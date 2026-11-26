//! Commands typed in the chat (`!status`, `!start 12`, `!stop 12`, `!approve 12`, `!tell 12 …`): the board runs them
//! at once, through the same actions and guards as the command line, and answers in the coordinator's chat with no turn
//! of its own (docs/chat-commands.md). Words that are no such command go to the coordinator unchanged.
use super::co::Co;
use crate::agents::talk::Said;
use crate::err::Res;
use crate::json::*;
use crate::store::keys::{repo_keys, say_ids};

/// What starts a command: no program's skill picker uses it (theirs start with `/` or `$`).
pub const PREFIX: char = '!';
const NAMES: [&str; 5] = ["status", "start", "stop", "approve", "tell"];
const USAGE: &str = "!status [id …] · !start <id> · !stop <id> · !approve <id> · !tell <id> <text>";
/// Of the commands answered, the last few stay in the chat.
const KEPT: usize = 6;

/// The command these words are: its name and what follows it. None: they are words for the coordinator.
fn parse(text: &str) -> Option<(&str, &str)> {
    let words = text.trim().strip_prefix(PREFIX)?;
    let (name, rest) = words.split_once(char::is_whitespace).unwrap_or((words, ""));
    NAMES.contains(&name).then(|| (name, rest.trim()))
}
/// The command run: the board's answer, or the refusal of its guards. A repo's coordinator's `!status` is of its repo.
fn answer(name: &str, args: &str, co: &Co) -> Res<String> {
    if name == "status" { return crate::task_status::status(&args.split_whitespace().map(String::from).collect::<Vec<_>>(), false, co.repo().unwrap_or("")).map(|rows| format!("```\n{}\n```", crate::task_status::said(&rows))); }
    let (id, text) = args.split_once(char::is_whitespace).map(|(i, t)| (i, t.trim())).unwrap_or((args, ""));
    if id.is_empty() || (name == "tell" && text.is_empty()) { crate::fail!("!{name} needs {}. The commands: {USAGE}", if id.is_empty() { "a task" } else { "the text" }); }
    // Another machine's exact actions on a task of this one (remote.rs) are the command line's, with its guards.
    let r = crate::remote::door("do", &json!({ "action": if name == "start" { "delegate" } else { name }, "id": id, "text": text }))?;
    if r.b("ok") { Ok(r.t("msg")) } else { crate::fail!("{}", r.s("msg")) }
}
/// Words that are a command: run, and kept with the answer for the chat. None: they go to the coordinator.
pub fn run(text: &str, co: &Co) -> Option<Res<Obj>> {
    let (name, args) = parse(text)?;
    let (reply, ok) = match answer(name, args, co) { Ok(m) => (m, true), Err(e) => (e.msg, false) };
    let entry = json!({ "said": text.trim(), "reply": say_ids(&reply, &repo_keys()), "ok": ok, "at": crate::clock::now() });
    Some(super::state::change(co, |c| {
        let mut l = c.arr("commands").to_vec();
        l.push(entry);
        l.drain(..l.len().saturating_sub(KEPT));
        c.put("commands", l);
        true
    }))
}
/// The commands answered lately, as the chat shows them: your words, then the board's answer.
pub fn talk(c: &Obj) -> impl Iterator<Item = Said> + '_ {
    c.arr("commands").iter().flat_map(|m| [Said { you: true, text: m.t("said"), at: m.i("at") }, Said { you: false, text: m.t("reply"), at: m.i("at") }])
}
