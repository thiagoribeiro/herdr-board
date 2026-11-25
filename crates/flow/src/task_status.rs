//! Compact, factual task snapshots shared by status (the command line's and the chat's `!status`), coordinator
//! context and operation receipts. No model is called.
use crate::err::Res;
use crate::json::*;
use crate::store::keys::{repo_keys, say_id};
use crate::store::tasks::{find, list, ListOpts};
use crate::text::{cut, pad};
use crate::store::reviews::current_review;

pub fn snapshot(t: &Obj) -> Value {
    let (owner, next) = next_step(t);
    let mut v = json!({ "id": t.at("id"), "title": t.at("title"), "stage": t.at("stage"),
        "state": crate::state::show(t, t.at("live"), crate::clock::now()).state, "owner": owner, "next": next, "at": t.at("at"),
        "ageMs": (crate::clock::now() - if t.b("stageSince") { t.i("stageSince") } else { t.i("at") }).max(0), "observedAt": crate::clock::now(),
        "availability": if t.at("live").is_object() { json!({ "status": t.at("live").at("status"), "down": t.at("live").b("down"), "gone": t.at("live").b("gone"), "away": t.at("live").at("away") }) } else { json!("saved") },
        "pr": t.at("pr"), "head": t.at("prHead"), "waiting": t.at("waiting"),
        "blocked": t.at("blocked"), "syncBlocked": t.at("syncBlocked"), "failure": if t.b("failure") { json!({ "kind": t.at("failure").at("kind"), "reason": crate::store::tasks::one_line(&text(t.at("failure")), 400) }) } else { Value::Null },
        "usage": t.at("spent").at("metrics"), "summary": crate::digest::cached(t) });
    if let Some(r) = current_review(t) {
        v["review"] = json!({ "head": r.at("head"), "meets": r.at("meets"), "text": crate::store::tasks::one_line(r.s("text"), 400),
            "findings": r.arr("findings").iter().map(|f| json!({ "id": f.at("id"), "severity": f.at("severity"), "subject": crate::store::tasks::one_line(f.s("subject"), 400) })).collect::<Vec<_>>() });
    }
    let status = crate::shown::turn::status_of(t, crate::clock::now());
    for k in ["state", "whose", "next", "since", "for", "stall"] { v[k] = status.at(k).clone(); }
    v.as_object_mut().unwrap().retain(|_, x| !x.is_null());
    v
}
pub fn next_step(t: &Obj) -> (&'static str, String) {
    let r = if t.b("state") { t.clone() } else { crate::flow::live::saved(t) };
    let status = crate::shown::turn::status_of(&r, crate::clock::now());
    let owner = if t.s("stage") == "done" { "none" } else if status.s("whose") == "agent" && t.s("stage") == "reviewing" { "reviewer" } else {
        match status.s("whose") { "you" => "human", "agent" => "agent", "github" => "github", _ => "board" }
    };
    (owner, status.t("next"))
}

/// The tasks named (by `,` too), else every task (`all`: with the done and archived ones); of `repo` when one is given.
pub fn tasks(ids: &[String], all: bool, repo: &str) -> Res<Vec<Obj>> {
    let found = if ids.is_empty() { list(ListOpts { include_archived: all, ..Default::default() }) } else { ids.iter().flat_map(|s| s.split(',')).map(find).collect::<Res<_>>()? };
    Ok(found.into_iter().filter(|t| repo.is_empty() || t.s("repo") == repo).collect())
}
/// What `herdr-board status` and the chat's `!status` say: a snapshot of each task named, else of each one not done
/// (`all`: every one), of `repo` when one is given, as it is live now.
pub fn status(ids: &[String], all: bool, repo: &str) -> Res<Vec<Value>> {
    let live = crate::flow::live::board(true);
    Ok(tasks(ids, all, repo)?.into_iter()
        .filter(|t| !ids.is_empty() || all || !(["done", "tried"].contains(&t.s("stage")) || t.b("archivedAt")))
        .map(|t| snapshot(live.iter().find(|r| r.s("id") == t.s("id")).unwrap_or(&t))).collect())
}
/// One short line per task: its key and short title, where it stands and for how long, whose turn it is and the next
/// step (shown/turn.rs). docs/status.md.
pub fn said(rows: &[Value]) -> String {
    if rows.is_empty() { return "No active tasks.".into(); }
    let keys = repo_keys();
    rows.iter().map(|r| format!("{} {} {} {} {}  {}", pad(&say_id(r.s("id"), &keys), 9), pad(&cut(r.s("title"), 32), 32), pad(r.s("state"), 16), pad(r.s("for"), 4), pad(r.s("whose"), 6), r.s("next"))).collect::<Vec<_>>().join("\n")
}
