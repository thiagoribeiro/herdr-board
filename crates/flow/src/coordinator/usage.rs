//! Incremental coordinator usage independent of limits, with per-request deltas and context-growth measurements.
use super::{co::Co, state::{get, change, home}};
use crate::agents::{adapter, read_tokens};
use crate::json::*;
pub fn count(co: &Co) -> Value {
    let Some(c) = get(co) else { return Value::Null };
    let kind = c.at("run").s("kind");
    let Some(file) = adapter::transcript(kind, c.s("session"), &home(co)) else { return c.at("usage").clone() };
    let name = file.display().to_string();
    let before = c.at("usageFiles").at(&name);
    let Some(counter) = read_tokens::tokens(kind, &file, before) else { return c.at("usage").clone() };
    if before.at("at") != counter.at("at") {
        let _ = change(co, |c| {
            if c.at("usageFiles").at(&name).f("at") > counter.f("at") { return false; }
            c.obj_mut("usageFiles").put(&name, counter);
            let model = c.at("usageFiles").at(&name).t("model");
            if !model.is_empty() { c.put("observedModel", model); }
            let context = c.at("usageFiles").at(&name).f("lastContext");
            if context > 0.0 { c.put("contextTokens", context); }
            let mut total = json!({});
            for f in c.at("usageFiles").as_object().into_iter().flat_map(|x| x.values()) {
                for k in ["input", "cacheWrite", "cacheRead", "output", "requests"] { total[k] = json!(total.f(k) + f.at("metrics").f(k)); }
            }
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
