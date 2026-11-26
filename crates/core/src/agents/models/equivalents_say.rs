//! Inspect declared counterpart families and role eligibility without starting an agent or probing a model with a prompt.
use super::{catalog, equivalents};
use crate::{err::Res, json::*};
pub fn command(name: &str) -> Res<(Value, String)> {
    let c = catalog::load();
    let selected = if name.is_empty() { None } else { Some(equivalents::find(&c, name).ok_or("No unambiguous catalog entry for this model; name its entry or its exact program/model")?) };
    let sources: Vec<_> = selected.map(|e| vec![e]).unwrap_or_else(|| c.entries.iter().filter(|e| !e.equivalents.is_empty()).collect());
    let mut rows = vec![];
    let mut lines = vec!["Corresponding models: declared task families, not benchmark-proven equal quality.".to_string()];
    for entry in sources {
        let targets: Vec<Value> = equivalents::candidates(&c, &entry.name).iter().map(|e| {
            let problem = crate::store::settings::values::models_fit("worker.agent", &json!(e.agent)).err().map(|e| e.msg);
            json!({ "entry": e.name, "agent": e.agent, "effort": e.effort, "family": e.family, "configured": problem.is_none(), "problem": problem })
        }).collect();
        lines.push(format!("{} [{}] {} · {} effort -> {}", entry.name, entry.family, entry.agent, entry.effort,
            if targets.is_empty() { "no declared counterpart".into() } else { targets.iter().map(|t| format!("{} ({} · {} effort{})", t.s("entry"), t.s("agent"), t.s("effort"), if t.b("configured") { String::new() } else { format!(" · {}", t.s("problem")) })).collect::<Vec<_>>().join(", ") }));
        rows.push(json!({ "entry": entry.name, "agent": entry.agent, "family": entry.family, "equivalents": targets }));
    }
    lines.push("Automatic task handover requires fallback.corresponding true. Explicit fallback lists win. Coordinator and helper programs keep their existing lifecycle.".into());
    if !c.problems.is_empty() { lines.push(format!("Not counted:\n{}", c.problems.join("\n"))); }
    Ok((json!({ "families": rows, "problems": c.problems, "validatedQuality": false }), lines.join("\n")))
}
