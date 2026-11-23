//! `herdr-board models catalog [--init]`: the catalog of models as the board reads it, each entry with its program,
//! effort, cost, what it is good for and what to avoid it for, where it escalates, and how it fared in the last 30
//! days; the programs allowed; the routing settings; and each entry that is refused, with why. `--init` writes the
//! board's example to `~/.herdr-board/models.toml`, for you to change.
use super::catalog::{load, path, EXAMPLE};
use super::stats::summary;
use crate::err::Res;
use crate::json::*;
use crate::store::marks::tilde;
use crate::store::settings::lookup::setting_text;
use crate::store::settings::values::models_fit;

pub fn init() -> Res<(Value, String)> {
    let p = path();
    if p.exists() { crate::refuse!("{} is there already: it is yours, and is not written over. Change it, or remove it first to start from the example.", tilde(&p.display().to_string())); }
    crate::store::files::write_atomic(&p, EXAMPLE.as_bytes())?;
    Ok((json!({ "file": p.display().to_string() }), format!("Wrote the example catalog to {}: change it to what you have (herdr-board models catalog shows it as read).", tilde(&p.display().to_string()))))
}
pub fn catalog_command(do_init: bool) -> Res<(Value, String)> {
    if do_init { return init(); }
    let mut c = load();
    // A model its program does not take is refused here, as `set` refuses it (its list is asked for only now).
    let mut refused = vec![];
    c.entries.retain(|e| match models_fit("worker.agent", &json!(e.agent)) { Ok(()) => true, Err(x) => { refused.push(format!("entry {}: {}; this entry is not used", e.name, x.msg.trim_start_matches("worker.agent: "))); false } });
    c.problems.extend(refused);
    let file = tilde(&path().display().to_string());
    let used = summary(&c);
    let mode = setting_text("route.mode", None);
    let mut lines = vec![
        if c.file { format!("Catalog of models: {file}") } else { format!("Catalog of models: the board's example ({file} is not there: herdr-board models catalog --init writes it, to change)") },
        format!("Programs allowed: {}", match &c.allow { Some(l) => format!("{} (allow.agents)", if l.is_empty() { "none".to_string() } else { l.join(", ") }), None => "all (no allow.agents)".into() }),
        format!("route.mode {mode} · route.default {} · route.escalateAfter {}", setting_text("route.default", None), setting_text("route.escalateAfter", None)),
        String::new(),
    ];
    let wide = c.entries.iter().map(|e| e.name.len()).max().unwrap_or(0);
    for e in &c.entries {
        let cost = if e.cost.fract() == 0.0 { format!("{}", e.cost as i64) } else { format!("{}", e.cost) };
        lines.push(format!("{:<wide$}  {} · effort {} · cost {cost}{}", e.name, e.agent, e.effort, if e.escalate_to.is_empty() { String::new() } else { format!(" · escalates to {}", e.escalate_to) }));
        let pad = " ".repeat(wide + 2);
        if !e.good_for.is_empty() { lines.push(format!("{pad}good for: {}", e.good_for)); }
        if !e.avoid_for.is_empty() { lines.push(format!("{pad}avoid for: {}", e.avoid_for)); }
        let signs: Vec<String> = [e.max_files.map(|n| format!("up to {n} files")), e.max_minutes.map(|n| format!("up to {n} minutes"))].into_iter().flatten().collect();
        if !signs.is_empty() { lines.push(format!("{pad}size: {}", signs.join(", "))); }
        lines.push(format!("{pad}used: {}", used.get(&e.name).cloned().unwrap_or_else(|| "not yet in 30 days".into())));
    }
    if c.entries.is_empty() { lines.push("No entries: nothing can be routed.".into()); }
    if !c.problems.is_empty() { lines.push(String::new()); lines.push("Not counted:".into()); lines.extend(c.problems.iter().map(|p| format!("  {p}"))); }
    let j = json!({ "file": path().display().to_string(), "fromFile": c.file, "allow": c.allow, "mode": mode, "entries": c.entries.iter().map(|e| { let mut o = obj(e.json()); o.put("used", used.get(&e.name).cloned().map(Value::String).unwrap_or(Value::Null)); Value::Object(o) }).collect::<Vec<_>>(), "problems": c.problems });
    Ok((j, lines.join("\n")))
}
