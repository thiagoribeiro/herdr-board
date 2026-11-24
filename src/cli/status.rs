//! `herdr-board status`: one short line per active task: its key and short title, where it stands and for how long,
//! whose turn it is and the next step (shown/turn.rs). Done and archived tasks only with `--all`. docs/status.md.
use crate::flow::live::board;
use crate::json::*;
use crate::store::keys::{repo_keys, say_id};
use crate::text::{cut, pad};

pub fn status(all: bool) -> (Value, String) {
    let (now, keys) = (crate::clock::now(), repo_keys());
    let rows: Vec<Value> = board(all).iter().filter(|t| all || !(t.b("archivedAt") || ["done", "tried"].contains(&t.s("stage")))).map(|t| crate::shown::turn::status_of(t, now)).collect();
    if rows.is_empty() { return (json!([]), "No active tasks.".into()); }
    let text = rows.iter().map(|r| format!("{} {} {} {} {}  {}", pad(&say_id(r.s("id"), &keys), 9), pad(&cut(r.s("title"), 32), 32), pad(r.s("state"), 16), pad(r.s("for"), 4), pad(r.s("whose"), 6), r.s("next"))).collect::<Vec<_>>().join("\n");
    (Value::Array(rows), text)
}
