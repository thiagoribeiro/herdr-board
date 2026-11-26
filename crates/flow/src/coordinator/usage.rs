//! Incremental coordinator usage independent of limits, with per-request deltas and context-growth measurements.
use super::{co::Co, state::{get, change, home}};
use crate::agents::{adapter, token_tree};
use crate::json::*;
pub fn count(co: &Co) -> Value {
    let Some(c) = get(co) else { return Value::Null };
    let kind = c.at("run").s("kind");
    let Some(file) = adapter::transcript(kind, c.s("session"), &home(co)) else {
        let _ = change(co, |now| {
            if now.s("session") != c.s("session") { return false; }
            if now.at("usageCoverage").b("partial") && now.at("usageCoverage").s("missingSession") == c.s("session") && !now.b("contextTokens") { return false; }
            if now.s("contextSession") != now.s("session") { now.del("observedModel"); }
            now.del("contextTokens"); now.del("contextSession");
            now.obj_mut("usageCoverage").put("partial", true);
            now.obj_mut("usageCoverage").put("missingSession", c.at("session").clone());
            true
        });
        return c.at("usage").clone();
    };
    let file = std::fs::canonicalize(&file).unwrap_or(file);
    let name = file.display().to_string();
    let mut files = c.o("usageFiles").cloned().unwrap_or_default();
    let roots: Vec<(String, String, String)> = files.iter().filter(|(p, f)| *p != &name && f.b("kind") && !f.b("parent"))
        .map(|(path, f)| (path.clone(), f.t("kind"), f.t("session"))).collect();
    let mut changed = false;
    for (path, kind, session) in roots { changed |= token_tree::count(&kind, std::path::Path::new(&path), &session, &mut files); }
    changed |= token_tree::count(kind, &file, c.s("session"), &mut files);
    let removed: Vec<(String, f64)> = c.o("usageFiles").into_iter().flat_map(|f| f.iter())
        .filter(|(p, _)| !files.contains_key(*p)).map(|(p, f)| (p.clone(), f.f("at"))).collect();
    if changed || c.s("contextSession") != c.s("session") {
        let _ = change(co, |c| {
            // A delayed reader cannot overwrite another conversation or a newer counter.
            if c.s("session") != files.at(&name).s("session") { return false; }
            if files.iter().any(|(p, f)| c.at("usageFiles").at(p).f("at") > f.f("at")) { return false; }
            if removed.iter().any(|(p, at)| c.at("usageFiles").at(p).f("at") > *at) { return false; }
            if c.s("session") != c.s("contextSession") {
                c.del("contextTokens"); c.del("observedModel");
            }
            for (path, _) in removed { c.obj_mut("usageFiles").del(&path); }
            for (path, counter) in files { c.obj_mut("usageFiles").put(&path, counter); }
            let model = c.at("usageFiles").at(&name).t("model");
            if !model.is_empty() { c.put("observedModel", model); }
            let context = c.at("usageFiles").at(&name).at("lastContext").clone();
            if context.is_number() { c.put("contextTokens", context); } else { c.del("contextTokens"); }
            c.put("contextSession", c.t("session"));
            let summary = token_tree::totals(c.o("usageFiles").unwrap());
            c.put("usageCoverage", json!({ "partial": summary.at("partial"), "sessions": summary.at("sessions"), "subagents": summary.at("subagents") }));
            let total = summary.at("metrics").clone();
            c.put("usage", total.clone());
            let base = c.at("usageBase").clone();
            let mut delta = json!({});
            for k in ["input", "cacheWrite", "cacheRead", "output", "requests"] { delta[k] = json!((total.f(k) - base.f(k)).max(0.0)); }
            let request = json!({ "requestId": c.at("typedId"), "tokens": delta, "observedAt": crate::clock::now() });
            c.put("requestUsage", request.clone());
            let mut history: Vec<Value> = c.arr("requestHistory").iter().filter(|r| r.at("requestId") != request.at("requestId")).cloned().collect();
            history.push(request); history.drain(..history.len().saturating_sub(50));
            c.put("requestHistory", history);
            true
        });
    }
    get(co).at("usage").clone()
}
