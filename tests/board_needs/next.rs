//! The key to the next task that needs you: it goes down Needs you in the list's order and round again, puts the reply
//! box on each, and never takes a draft or the task it is for when the list moves under it. With `board.nextAfterReply`
//! an answer moves on by itself. A row that waits says why first, and its time counts from when its agent stopped.
use crate::common;
use common::fixtures::fixture_rows;
use common::*;
use serde_json::json;
use sw::board::data::Data;
use sw::board::job::Job;
use sw::board::keys::{chord, Press, K};
use sw::board::state::{Ctx, State};
use sw::board::update::{list::relist, update, Effect, Event};
use sw::json::*;

const NOW: i64 = 1_790_000_000_000;
/// The frames' rows, each task that waits for you with an agent of its own to answer; #5 (working) selected.
fn board(w: &World) -> State {
    let id = w.add("Max length for task titles");
    let items = fixture_rows(&task(&id), NOW).into_iter().map(|(_, mut r)| { if r.s("state") != "working" { r.put("link", json!({ "pane": "p1" })); } std::sync::Arc::new(r) }).collect();
    let data = Data { items, loaded: true, session: "default".into(), repo: "proj".into(), repo_root: w.root_str(), ..Default::default() };
    let mut s = State::new(Ctx { cwd: w.root_str(), ..Default::default() }, data);
    s.now = NOW; s.sel = "proj/5".into();
    relist(&mut s);
    s
}
fn next() -> Press { Press::chord(&chord(K::NextNeeds)) }
fn key(s: &mut State, p: Press) -> Vec<Effect> { update(s, Event::Key(p)) }
fn typed(s: &mut State, text: &str) { for c in text.chars() { key(s, Press::char(c)); } }
fn told(fx: &[Effect]) -> Vec<String> { fx.iter().filter_map(|e| if let Effect::Run(Job::Tell { id, .. }) = e { Some(id.clone()) } else { None }).collect() }

#[test]
fn the_key_goes_down_needs_you_in_the_lists_order_round_again_and_the_chat_talks_to_each() {
    let w = world();
    let mut s = board(&w);
    let mut went = vec![];
    for _ in 0..6 { key(&mut s, next()); assert!(s.reply && s.reply_id == s.sel, "the reply box is on it"); went.push(s.sel.clone()); }
    assert_eq!(went, ["proj/9", "proj/10", "proj/12", "proj/13", "proj/14", "proj/9"]);
    // what you type next is the answer to it
    typed(&mut s, "Keep it");
    let fx = key(&mut s, Press::chord("enter"));
    assert_eq!((told(&fx), s.sel.as_str(), s.value()), (vec!["proj/9".to_string()], "proj/9", String::new()), "board.nextAfterReply is off: the board stays");
}

#[test]
fn with_next_after_reply_an_answer_moves_on_to_the_following_one() {
    let w = world();
    let mut s = board(&w);
    s.data.next_after_reply = true;
    key(&mut s, next()); typed(&mut s, "Keep it");
    let fx = key(&mut s, Press::chord("enter"));
    assert_eq!(told(&fx), ["proj/9"]);
    assert!(s.sel == "proj/10" && s.reply && s.reply_id == "proj/10" && s.value().is_empty());
    // a reply to a task that does not wait for you stays where it is
    s.sel = "proj/5".into(); relist(&mut s); s.reply = true; s.reply_id = "proj/5".into();
    typed(&mut s, "carry on"); key(&mut s, Press::chord("enter"));
    assert_eq!(s.sel, "proj/5");
}

#[test]
fn a_draft_holds_the_key_and_the_list_moving_never_moves_whom_it_is_for() {
    let w = world();
    let mut s = board(&w);
    key(&mut s, next()); key(&mut s, next());
    typed(&mut s, "Sign in later");
    // with words typed the key is the text box's: nothing moves, nothing is lost
    key(&mut s, next());
    assert_eq!((s.sel.as_str(), s.reply_id.as_str(), s.value()), ("proj/10", "proj/10", "Sign in later".to_string()));
    // the list reorders under it (#9 is answered and goes to work; #10 is now first): the draft stays with #10
    s.data.items = s.data.items.iter().map(|r| { let mut o = (**r).clone(); if o.s("id") == "proj/9" { o.put("state", "working"); o.put("stage", "working"); } std::sync::Arc::new(o) }).collect();
    relist(&mut s);
    assert_eq!((s.sel.as_str(), s.reply_id.as_str(), s.value(), s.reply), ("proj/10", "proj/10", "Sign in later".to_string(), true));
    let fx = key(&mut s, Press::chord("enter"));
    assert_eq!(told(&fx), ["proj/10"]);
    // #9 no longer waits: the key goes on from #10 and never back to it
    let mut went = vec![];
    for _ in 0..4 { key(&mut s, next()); went.push(s.sel.clone()); }
    assert_eq!(went, ["proj/12", "proj/13", "proj/14", "proj/10"]);
}

#[test]
fn only_what_the_list_shows_a_filter_keeps_what_it_hides() {
    let w = world();
    let mut s = board(&w);
    s.show = sw::board::update::shows::SHOWS.iter().position(|(n, _)| *n == "Working").unwrap(); relist(&mut s);
    key(&mut s, next());
    assert_eq!((s.sel.as_str(), s.reply, s.msg.as_str()), ("proj/5", false, "Nothing in view needs you."));
    // under Needs you the selection lands on #9: the key answers that one first, then goes on
    s.show = sw::board::update::shows::SHOWS.iter().position(|(n, _)| *n == "Needs you").unwrap(); relist(&mut s);
    assert!(s.sel == "proj/9" && !s.reply);
    key(&mut s, next()); assert!(s.sel == "proj/9" && s.reply_id == "proj/9" && s.reply);
    key(&mut s, next()); assert_eq!(s.sel, "proj/10");
}

#[test]
fn a_blocked_rows_time_counts_from_when_its_agent_stopped_to_wait() {
    let t = |last: serde_json::Value| { let r = sw::shown::states::as_row(&obj(json!({ "id": "proj/1", "state": "blocked", "stage": "working", "at": 1000, "last": last }))); r.i("at") };
    assert_eq!(t(json!({ "status": "blocked", "at": 5000 })), 5000, "blocked after its stage began: since then");
    assert_eq!(t(json!({ "status": "working", "at": 5000 })), 1000, "herdr saw it work last: since its stage changed");
    assert_eq!(t(json!({ "status": "blocked", "at": 500 })), 1000, "blocked before its stage changed: since the stage");
}
