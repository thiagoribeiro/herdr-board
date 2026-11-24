//! The programs you allow (`allow.agents` in models.toml, docs/models.md#catalog-and-routing): with the list, a program
//! that is not on it is refused wherever it is named (a setting, a fallback, `attempts.by`, `--worker.agent`, an entry
//! of the catalog), and a built-in default that names one (`reviewer.agent codex`) is refused when it would start.
//! Without the list every program is allowed.
use super::catalog::{path, raw};
use crate::store::marks::tilde;

/// The programs allowed, as the catalog's file says them; None: every program.
pub fn allowed() -> Option<Vec<String>> { raw().allow.clone() }
/// Is this program allowed here?
pub fn allows(kind: &str) -> bool { allowed().map(|l| l.iter().any(|k| k == kind)).unwrap_or(true) }
/// Why this program is not allowed here, in words that follow its name; None when it is.
fn why(kind: &str) -> Option<String> {
    let list = allowed()?;
    if list.iter().any(|k| k == kind) { return None; }
    Some(format!("{kind} is not allowed here: allow.agents in {} allows only {}", tilde(&path().display().to_string()), if list.is_empty() { "no program".to_string() } else { list.join(", ") }))
}
/// Why `setting` may not name this program here; None when it may.
pub fn refusal(setting: &str, kind: &str) -> Option<String> {
    why(kind).map(|w| format!("{setting}: {w}. Choose one of those, or add {kind} to that list."))
}
/// The same for a program about to start, whatever named it (a built-in default too, such as `reviewer.agent codex`).
pub fn start_refusal(kind: &str) -> Option<String> {
    why(kind).map(|w| format!("{w}, so it is not started. `herdr-board show` names the setting that chose it, a built-in one too: set that to one of those, or add {kind} to that list."))
}
/// What `show` says beside a setting whose value names a program that is not allowed here.
pub fn note(kind: &str) -> String { why(kind).map(|w| format!("refused when it starts: {w}")).unwrap_or_default() }
