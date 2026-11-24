//! Persistent task results, summarized once by a small helper and reused until their evidence changes. Status stays live.
pub mod prompt;
pub mod evidence;
pub mod run;
use crate::json::*;
use crate::store::reviews::current_review;

pub const VERSION: i64 = 1;
/// Only delivery evidence belongs in the cache key. Poll times, progress, stage and merges do not invalidate it.
pub fn source(t: &Obj) -> Value {
    let review = current_review(t).map(|r| json!({ "head": r.at("head"), "meets": r.at("meets"), "findings": r.at("findings"),
        "text": r.at("text"), "checks": r.at("checks"), "limitations": r.at("limitations"), "artifacts": r.at("artifacts") })).unwrap_or(Value::Null);
    json!({ "version": VERSION, "id": t.at("id"), "title": t.at("title"), "requirements": t.at("description"),
        "delivery": t.at("summary"), "reportFile": t.at("report").at("file"), "reportDocument": evidence::report(t), "head": t.at("prHead"),
        "check": t.at("check"), "checkPassed": t.at("checkPassed"), "checkShort": t.at("checkShort"), "review": review })
}
pub fn key(t: &Obj) -> String {
    let mut hash = sha1_smol::Sha1::from(source(t).to_string());
    hash.update(prompt::SYSTEM.as_bytes());
    hash.update(prompt::prompt(&Value::Null).as_bytes());
    hash.digest().to_string()
}
pub fn cached(t: &Obj) -> Value {
    let d = t.at("digest");
    if d.s("key") == key(t) && d.at("body").is_object() { d.at("body").clone() } else { Value::Null }
}
pub fn ready(t: &Obj) -> bool {
    !t.b("archivedAt") && !t.b("attemptOf") && (t.b("summary") || t.b("report")) &&
        ["approval", "done", "failed", "reviewneeds", "nosummary"].contains(&t.s("stage"))
}
/// Deterministic references are added by the board, never invented by the helper.
pub fn result(t: &Obj, mut body: Value) -> Value {
    let mut observed = vec![];
    if t.at("checkPassed").b("head") { observed.push(format!("Board check passed on {}: {}", t.at("checkPassed").s("head"), t.at("checkPassed").s("with"))); }
    if let Some(r) = current_review(t) { observed.push(format!("Independent review on {}: meets={}, {} open findings", r.s("head"), r.b("meets"), r.arr("findings").len())); }
    body["observed"] = json!(observed);
    json!({ "key": key(t), "version": VERSION, "at": crate::clock::now(), "body": body,
        "evidence": evidence::references(t) })
}
/// A result can be displayed even if its coordinator is asleep. This never queues a model turn.
pub fn messages(co: &crate::coordinator::co::Co) -> Vec<crate::agents::talk::Said> {
    use crate::store::tasks::{list, ListOpts};
    let mut out = vec![];
    for t in list(ListOpts::default()) {
        let socket = t.at("link").s("socket");
        if (!socket.is_empty() && !crate::herdr::cli::same_socket(socket, &co.socket)) ||
            t.so("coordinator").unwrap_or("main") != co.settings_key() { continue; }
        let body = cached(&t);
        if body.is_null() && !ready(&t) { continue; }
        let delivery = if body.is_null() { if t.b("summary") { t.t("summary") } else { t.at("report").t("link") } } else { prompt::display(&body) };
        if delivery.is_empty() { continue; }
        let at = if body.is_null() { if t.b("workedAt") { t.i("workedAt") } else { t.i("at") } } else { t.at("digest").i("at") };
        out.push(crate::agents::talk::Said { you: false, at,
            text: format!("{} · Task result\n{}\n{}", t.s("id"), delivery,
                crate::task_status::next_step(&t).1) });
    }
    out.sort_by_key(|m| m.at);
    out.drain(..out.len().saturating_sub(12));
    out
}
