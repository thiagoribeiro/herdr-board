//! Evidence-backed trial records and paired comparisons for opt-in evaluations of agent quality, usage and friction.
pub mod cases;
use crate::{err::Res, json::*};
use std::collections::BTreeMap;
pub const VERSION: i64 = 1;

/// An assessment is supplied by a human or an explicitly named evaluator, never inferred from the worker's summary.
pub fn record(task: &Obj, assessment: &Value) -> Res<Value> {
    let case = cases::find(assessment.s("case"))?;
    for k in ["variant", "input", "assessedBy"] {
        if assessment.s(k).trim().is_empty() { crate::fail!("Assessment needs nonempty {k}"); }
    }
    for k in ["trial", "elapsedMs", "interruptions", "escapedDefects", "unauthorizedActions"] {
        if assessment.at(k).as_u64().is_none() || (k == "trial" && assessment.i(k) == 0) { crate::fail!("Assessment needs a nonnegative integer {k} (trial starts at 1)"); }
    }
    let criteria = assessment.at("criteria");
    if criteria.as_object().map(|o| o.len()) != Some(case.criteria.len()) { crate::fail!("Assess every criterion of {} exactly once", case.id); }
    for (id, _) in case.criteria {
        if !criteria.at(id).is_boolean() || assessment.at("evidence").s(id).trim().is_empty() { crate::fail!("Criterion {id} needs a boolean result and original evidence"); }
    }
    if !["approval", "done", "stopped", "failed", "asks", "plan", "pick"].contains(&task.s("stage")) { crate::fail!("Wait for this task's execution to end before recording a trial"); }
    let blocked = task.s("stage") == "failed" || task.b("failure") || task.b("prFailure") || task.b("mergeFailed") || crate::store::reviews::review_result(task) == "fail";
    let success = !blocked && criteria.as_object().unwrap().values().all(|v| v == &json!(true)) && assessment.i("escapedDefects") == 0 && assessment.i("unauthorizedActions") == 0;
    let spent = task.at("spent");
    let measured = !spent.b("partial") && spent.at("files").as_object().is_some_and(|f| !f.is_empty());
    Ok(json!({ "version": VERSION, "case": case.id, "variant": assessment.at("variant"), "input": assessment.at("input"),
        "trial": assessment.at("trial"), "task": task.at("id"), "observedAt": crate::clock::now(), "success": success,
        "assessment": assessment, "usage": { "tokens": if measured { spent.at("total").clone() } else { Value::Null },
            "partial": !measured, "metrics": spent.at("metrics"), "sessions": spent.at("sessions"), "subagents": spent.at("subagents"),
            "scope": "task transcripts including native subagents; coordinator and shared helpers excluded", "summarizer": task.at("digestUsage") },
        "delivery": { "stage": task.at("stage"), "reviewedHead": task.at("reviewedHead"), "checkedHead": task.at("checkedHead"),
            "checked": task.at("checked"), "review": task.at("review"), "reviews": task.at("reviews"), "failure": task.at("failure"),
            "prFailure": task.at("prFailure"), "mergeFailed": task.at("mergeFailed") } }))
}

fn keyed(rows: &[Value]) -> Res<BTreeMap<String, &Value>> {
    let mut out = BTreeMap::new();
    let mut variant = None;
    for r in rows {
        if r.i("version") != VERSION || !r.at("success").is_boolean() { crate::fail!("Unsupported or invalid evaluation record"); }
        cases::find(r.s("case"))?;
        let mut task = obj(r.at("delivery").clone()); task.put("id", r.at("task").clone());
        let checked = record(&task, r.at("assessment"))?;
        for k in ["case", "variant", "input", "trial", "success"] {
            if checked.at(k) != r.at(k) { crate::fail!("Evaluation record {k} disagrees with its assessment or delivery evidence"); }
        }
        let usage = r.at("usage");
        if !usage.at("partial").is_boolean() || (!usage.b("partial") && !usage.at("tokens").as_f64().is_some_and(|n| n.is_finite() && n >= 0.0)) { crate::fail!("Usage must be explicitly partial or contain measured nonnegative tokens"); }
        if let Some(v) = variant { if v != r.s("variant") { crate::fail!("Each comparison side must contain one variant"); } }
        variant = Some(r.s("variant"));
        if r.s("input").is_empty() || r.i("trial") < 1 { crate::fail!("A record needs its original input identity and trial number"); }
        let key = json!([r.at("case"), r.at("input"), r.at("trial")]).to_string();
        if out.insert(key, r).is_some() { crate::fail!("Repeated case/input/trial would bias the comparison"); }
    }
    Ok(out)
}
/// Compare identical inputs and trials. Missing usage never becomes zero, and unmatched trials never disappear.
pub fn compare(baseline: &[Value], candidate: &[Value]) -> Res<Value> {
    let (a, b) = (keyed(baseline)?, keyed(candidate)?);
    if a.is_empty() || b.is_empty() { crate::fail!("Both variants need recorded trials"); }
    if a.keys().ne(b.keys()) { crate::fail!("Compare the same cases, input identities and trial numbers; trials are missing on one side"); }
    if baseline[0].s("variant") == candidate[0].s("variant") { crate::fail!("Name two different variants"); }
    let mut regressions = vec![];
    let mut tokens = [0.0, 0.0]; let mut measured = true;
    let mut successes = [0, 0]; let mut interruptions = [0, 0]; let mut elapsed = [0, 0];
    let mut unsafe_actions = 0; let mut escaped = 0;
    for (key, old) in &a {
        let new = b[key];
        if old.b("success") && !new.b("success") { regressions.push(json!({ "case": new.at("case"), "input": new.at("input"), "trial": new.at("trial") })); }
        for (i, row) in [*old, new].iter().enumerate() {
            successes[i] += i64::from(row.b("success"));
            interruptions[i] += row.at("assessment").i("interruptions");
            elapsed[i] += row.at("assessment").i("elapsedMs");
            measured &= !row.at("usage").b("partial") && row.at("usage").at("tokens").as_f64().is_some();
            tokens[i] += row.at("usage").f("tokens");
        }
        unsafe_actions += new.at("assessment").i("unauthorizedActions");
        escaped += new.at("assessment").i("escapedDefects");
    }
    let complete = cases::CASES.iter().all(|case| {
        let trials: Vec<_> = candidate.iter().filter(|r| r.s("case") == case.id).collect();
        !trials.is_empty() && trials.iter().all(|r| candidate.iter().filter(|other| other.s("case") == case.id && other.s("input") == r.s("input")).count() >= 3)
    });
    let quality = regressions.is_empty() && unsafe_actions == 0 && escaped == 0 && successes[1] >= successes[0];
    Ok(json!({ "version": VERSION, "baseline": baseline[0].at("variant"), "candidate": candidate[0].at("variant"), "pairs": a.len(),
        "successes": successes, "interruptions": interruptions, "elapsedMs": elapsed, "regressions": regressions,
        "unauthorizedActions": unsafe_actions, "escapedDefects": escaped, "suiteComplete": complete, "qualityPreserved": quality,
        "taskTokens": if measured { json!(tokens) } else { Value::Null },
        "tokenReduction": if measured && tokens[0] > 0.0 { json!(1.0 - tokens[1] / tokens[0]) } else { Value::Null },
        "readyForTaskUsageDecision": complete && quality && measured && successes[1] == a.len() as i64,
        "scope": "paired task usage only; coordinator and shared helper overhead requires separate measurement" }))
}
