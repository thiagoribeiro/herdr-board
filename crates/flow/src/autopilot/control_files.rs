//! Human approval holds for native agent instructions, permissions, hooks and MCP configuration changed by autopilot.
use crate::{err::Res, json::*, store::repos::git_out};
use std::path::Path;

pub fn protected(path: &str) -> bool {
    let parts: Vec<&str> = path.split('/').collect();
    let name = parts.last().copied().unwrap_or("");
    ["AGENTS.md", "CLAUDE.md", "CLAUDE.local.md", ".mcp.json"].contains(&name)
        || parts.iter().any(|p| [".claude", ".codex", ".agents"].contains(p))
}
pub fn changed(t: &Obj) -> Res<Vec<String>> {
    let tree = t.s("worktree");
    if tree.is_empty() { return Ok(vec![]); }
    let at = Path::new(tree);
    if !at.is_dir() { crate::fail!("The worktree is unavailable; agent control files could not be checked"); }
    let onto = crate::sync::lands_on(t);
    let base = [format!("origin/{onto}"), onto, t.t("baseSha")].into_iter().filter(|r| !r.is_empty())
        .find(|r| git_out(at, &["rev-parse", "--verify", "--quiet", &format!("{r}^{{commit}}")]).is_ok())
        .ok_or("The comparison base is unavailable; agent control files could not be checked")?;
    let mut paths = vec![];
    for args in [vec!["diff", "--name-only", "--no-renames", "-z", &format!("{base}...HEAD")],
        vec!["diff", "--name-only", "--no-renames", "-z", "HEAD"], vec!["ls-files", "--others", "--exclude-standard", "-z"]] {
        let raw = git_out(at, &args)?;
        for name in raw.split('\0').filter(|s| protected(s)) { if !paths.iter().any(|p| p == name) { paths.push(name.to_string()); } }
    }
    paths.sort(); Ok(paths)
}
pub fn touches(t: &Obj) -> bool { changed(t).map(|files| !files.is_empty()).unwrap_or(true) }
pub fn held(t: &Obj) -> bool {
    t.at("autopilot").is_object() && t.at("autopilot").at("controlFilesWaits") != &json!(false) && touches(t)
}
