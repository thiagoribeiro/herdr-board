//! `herdr-board status`: one short line per active task: its key and short title, where it stands and for how long,
//! whose turn it is and the next step (shown/turn.rs). Done and archived tasks only with `--all`. docs/status.md.
use crate::json::*;
use crate::store::keys::{repo_keys, say_id};
use crate::text::{cut, pad};

pub fn line(r: &Value) -> String {
    format!("{} {} {} {} {}  {}", pad(&say_id(r.s("id"), &repo_keys()), 9), pad(&cut(r.s("title"), 32), 32), pad(r.s("state"), 16), pad(r.s("for"), 4), pad(r.s("whose"), 6), r.s("next"))
}
