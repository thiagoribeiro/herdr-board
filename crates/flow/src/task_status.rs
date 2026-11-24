//! Compact, factual task snapshots shared by status, coordinator context and operation receipts. No model is called.
use crate::json::*;
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

pub fn line(v: &Value) -> String { format!("{}  {}  {}  {}: {}", v.s("id"), v.s("title"), v.s("stage"), v.s("owner"), v.s("next")) }
