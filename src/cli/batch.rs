//! Multi-task operations with persistent per-item receipts. Repeated request ids never repeat a side effect.
use super::flags::Args;
use crate::err::Res;
use crate::json::*;
use crate::store::{files::{data_dir, read_json, with_lock_wait, write_json}, tasks::find};
pub fn command(a: &Args) -> Res<(Value, String)> {
    if a.flags.has("request-id") && a.flags.s("request-id").trim().is_empty() { crate::fail!("--request-id needs a nonempty id"); }
    let action = a.pos.get(1).map(String::as_str).unwrap_or("");
    if !["delegate", "approve", "stop", "retry", "review", "fix", "archive", "restore"].contains(&action) {
        crate::fail!("batch takes delegate, approve, stop, retry, review, fix, archive or restore, followed by task ids");
    }
    let ids: Vec<String> = a.pos.iter().skip(2).flat_map(|s| s.split(',')).filter(|s| !s.is_empty()).map(String::from).collect();
    if ids.is_empty() { crate::fail!("batch needs at least one task id"); }
    // Validate every id before changing any task. Resolve keys now, so a later key rename cannot change the receipt.
    let mut tasks = vec![];
    for id in ids { let t = find(&id)?; if !tasks.iter().any(|x: &Obj| x.s("id") == t.s("id")) { tasks.push(t); } }
    let request = a.flags.so("request-id").map(String::from).unwrap_or_else(crate::util::uuid);
    let mut flags = a.flags.clone(); flags.del("request-id"); flags.del("json");
    let probe = Args { pos: vec![action.into()], flags: flags.clone(), typed: a.typed.clone() };
    super::flags::refuse_unknown(action, &probe)?;
    let signature = json!({ "action": action, "ids": tasks.iter().map(|t| t.t("id")).collect::<Vec<_>>(), "flags": flags });
    let name = crate::coordinator::rules_news::hash(&request);
    let file = data_dir().join("operations").join(format!("{name}.json"));
    with_lock_wait(&format!("operation:{name}"), 0, || {
        let old = read_json(&file);
        let replayed = old.is_some();
        if old.as_ref().is_some_and(|v| v.at("signature") != &signature) { crate::fail!("request-id was already used for different work; use a new request-id"); }
        let mut receipt = old.unwrap_or_else(|| json!({ "requestId": request, "signature": signature, "results": [], "at": crate::clock::now() }));
        for t in &tasks {
            if receipt.arr("results").iter().any(|r| r.s("id") == t.s("id")) { continue; }
            // A crash after this write is deliberately not retried: its outcome must be inspected, not guessed.
            if receipt.b("inFlight") { crate::fail!("Previous operation has an uncertain outcome for {}; inspect status before issuing a new request", receipt.s("inFlight")); }
            receipt["inFlight"] = t.at("id").clone(); write_json(&file, &receipt)?;
            let next = Args { pos: vec![action.into(), t.t("id")], flags: flags.clone(), typed: flags.clone() };
            let result = super::tasks::command(action, &next);
            let row = match result {
                Ok((value, message)) => json!({ "id": t.at("id"), "ok": true, "accepted": true, "message": message, "result": value,
                    "current": find(t.s("id")).ok().map(|t| crate::task_status::snapshot(&t)) }),
                Err(e) => json!({ "id": t.at("id"), "ok": false, "error": e.msg }),
            };
            receipt["results"].as_array_mut().unwrap().push(row);
            receipt["inFlight"] = Value::Null; write_json(&file, &receipt)?;
        }
        receipt["ok"] = json!(receipt.arr("results").iter().all(|r| r.b("ok")));
        receipt["complete"] = json!(true); write_json(&file, &receipt)?;
        receipt["replayed"] = json!(replayed);
        if replayed { for row in receipt["results"].as_array_mut().unwrap() { if let Ok(t) = find(row.s("id")) { row["current"] = crate::task_status::snapshot(&t); } } }
        let mut words = receipt.arr("results").iter().map(|r| if r.b("ok") { r.t("message") } else { format!("{}: {}", r.s("id"), r.s("error")) }).collect::<Vec<_>>().join("\n");
        if replayed { words = format!("Saved receipt for {request}:\n{words}"); }
        Ok((receipt, words))
    })
}
