//! Incremental usage of native agent families, following verified child sessions and retaining late or missing evidence.
use super::{adapter, read_tokens};
use crate::json::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

fn metadata(file: &Path) -> Value {
    let head = super::read_turn::head_of(file, 400000);
    head.lines().next().and_then(|l| serde_json::from_str::<Value>(l).ok()).unwrap_or(Value::Null)
}
fn parent_of(meta: &Value) -> &str {
    let p = meta.at("payload");
    p.so("parent_thread_id").unwrap_or_else(|| p.at("source").at("subagent").at("thread_spawn").s("parent_thread_id"))
}
fn regular(file: &Path) -> bool { std::fs::symlink_metadata(file).is_ok_and(|m| m.file_type().is_file()) }

/// Read a root and its descendants into the same file map. Counters survive a review closing or a worker switching.
pub fn count(kind: &str, file: &Path, session: &str, files: &mut Obj) -> bool {
    let mut pending = vec![(file.to_path_buf(), session.to_string(), String::new())];
    let mut visited = HashSet::new();
    let mut changed = false;
    while let Some((file, session, parent)) = pending.pop() {
        let file = std::fs::canonicalize(&file).unwrap_or(file);
        let parent = if parent.is_empty() { parent } else { std::fs::canonicalize(&parent).map(|p| p.display().to_string()).unwrap_or(parent) };
        let path = file.display().to_string();
        if !visited.insert(path.clone()) { continue; }
        let aliases: Vec<String> = files.keys().filter(|p| *p != &path && std::fs::canonicalize(p).is_ok_and(|real| real == file)).cloned().collect();
        for alias in aliases {
            if let Some(counter) = files.shift_remove(&alias) {
                if !files.contains_key(&path) || counter.f("at") > files.at(&path).f("at") { files.insert(path.clone(), counter); }
                changed = true;
            }
        }
        let before = files.get(&path).cloned().unwrap_or(Value::Null);
        let Some(mut counter) = read_tokens::tokens(kind, &file, &before) else {
            let mut unavailable = before.clone();
            if !unavailable.is_object() { unavailable = json!({}); }
            unavailable["kind"] = json!(kind); unavailable["session"] = json!(session); unavailable["parent"] = json!(parent);
            unavailable["unavailable"] = json!(true);
            if unavailable != before { files.insert(path, unavailable); changed = true; }
            continue;
        };
        let mut children = before.arr("children").to_vec();
        let mut missing = vec![];
        let rows = read_tokens::grown(&file, before.f("at") as u64).map(|(rows, _)| rows).unwrap_or_default();
        match kind {
            "claude" => {
                let dir = file.with_extension("").join("subagents");
                let linked_dir = std::fs::symlink_metadata(&dir).is_ok_and(|m| m.file_type().is_symlink());
                for row in &rows {
                    if row.s("type") != "user" { continue; }
                    for part in row.at("message").arr("content").iter().filter(|p| p.s("type") == "tool_result") {
                        for m in crate::re!(r"agentId: ([A-Za-z0-9_-]+)").captures_iter(&part.at("content").to_string()) {
                            let id = &m[1];
                            let item = json!({ "session": id, "file": dir.join(format!("agent-{id}.jsonl")) });
                            if !children.iter().any(|c| c.s("session") == id) { children.push(item); }
                        }
                    }
                }
                if linked_dir { missing.push(json!(dir)); }
                match if linked_dir { Err(std::io::Error::from(std::io::ErrorKind::NotFound)) } else { std::fs::read_dir(&dir) } {
                    Ok(entries) => for entry in entries.flatten() {
                        let child = entry.path();
                        let name = entry.file_name().to_string_lossy().into_owned();
                        if name.starts_with("agent-") && name.ends_with(".jsonl") && regular(&child) {
                            let name = name.trim_start_matches("agent-").trim_end_matches(".jsonl");
                            let item = json!({ "session": name, "file": child });
                            if !children.iter().any(|c| c.s("session") == name) { children.push(item); }
                        }
                    },
                    Err(e) if e.kind() != std::io::ErrorKind::NotFound => missing.push(json!(dir)),
                    _ => {}
                }
                for child in &children {
                    let path = PathBuf::from(child.s("file"));
                    if !linked_dir && regular(&path) { pending.push((path, child.t("session"), file.display().to_string())); }
                    else { missing.push(json!(child.s("file"))); }
                }
            }
            "codex" => {
                // Read only new complete lines; parentage comes from metadata, never a quoted task's text.
                for row in rows {
                    let item = row.at("payload").at("item");
                    if row.s("type") == "event_msg" && item.s("type") == "SubAgentActivity" && item.s("kind") == "started" {
                        let id = item.t("agent_thread_id");
                        if !id.is_empty() && !children.iter().any(|c| c.s("session") == id) { children.push(json!({ "session": id })); }
                    }
                }
                let own = metadata(&file).at("payload").so("id").unwrap_or(&session).to_string();
                for child in &mut children {
                    let path = child.so("file").map(PathBuf::from).filter(|p| regular(p))
                        .or_else(|| adapter::transcript("codex", child.s("session"), Path::new("")));
                    let Some(path) = path.filter(|p| regular(p)) else { missing.push(json!(child.s("session"))); continue };
                    let meta = metadata(&path);
                    if parent_of(&meta) != own || meta.at("payload").s("id") != child.s("session") {
                        missing.push(json!(child.s("session"))); continue;
                    }
                    child["file"] = json!(path);
                    pending.push((path, child.t("session"), file.display().to_string()));
                }
            }
            _ => {}
        }
        counter["kind"] = json!(kind); counter["session"] = json!(session); counter["parent"] = json!(parent);
        counter["children"] = json!(children); counter["missingChildren"] = json!(missing);
        counter = tidy(counter);
        if before != counter { files.insert(path, counter); changed = true; }
    }
    changed
}

/// Shared Claude message ids can occur in copied history: keep the greatest observed usage for each reply once.
pub fn totals(files: &Obj) -> Value {
    let mut total = 0.0;
    let mut metrics = json!({});
    let mut replies = Obj::new();
    let mut partial = false;
    for file in files.values() {
        partial |= file.b("partial") || !file.b("kind") || !file.at("metrics").is_object() || file.b("unavailable") || !file.arr("missingChildren").is_empty();
        let seen = file.o("seen");
        let accounted: f64 = seen.into_iter().flat_map(|m| m.values()).map(|p| sum(p)).sum();
        if file.s("kind") == "claude" && seen.is_some() && (accounted - file.f("n")).abs() < 0.5 {
            for (id, parts) in seen.unwrap() {
                let reply = replies.entry(id.clone()).or_insert_with(|| json!({}));
                for k in ["input", "cacheWrite", "cacheRead", "output"] { reply[k] = json!(reply.f(k).max(parts.f(k))); }
            }
        } else {
            total += file.f("n");
            for k in ["input", "cacheWrite", "cacheRead", "output", "requests"] { metrics[k] = json!(metrics.f(k) + file.at("metrics").f(k)); }
        }
    }
    for parts in replies.values() {
        total += sum(parts);
        for k in ["input", "cacheWrite", "cacheRead", "output"] { metrics[k] = json!(metrics.f(k) + parts.f(k)); }
        metrics["requests"] = json!(metrics.i("requests") + 1);
    }
    tidy(json!({ "total": total, "metrics": metrics, "partial": partial, "sessions": files.len(),
        "subagents": files.values().filter(|f| f.b("parent")).count() }))
}
fn sum(parts: &Value) -> f64 { ["input", "cacheWrite", "cacheRead", "output"].iter().map(|k| parts.f(k)).sum() }
