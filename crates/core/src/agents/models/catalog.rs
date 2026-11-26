//! The catalog of models (`~/.herdr-board/models.toml`, docs/models.md#catalog-and-routing): the entries a task can
//! be routed to, each a program with its model, an effort, a relative cost, what it is good for and what to avoid it
//! for (free words the Coordinator reads), and the entry it escalates to. The file is yours; without it the board's
//! example (models.example.toml) is the catalog. Each entry is checked as `set worker.agent` checks a value: an entry
//! that is refused is a problem, named, and is not used.
use crate::json::*;
use crate::store::files::home_dir;
use crate::store::settings::values::{check, fits};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// The board's example, and the catalog while you have no file of your own.
pub const EXAMPLE: &str = include_str!("models.example.toml");
pub fn path() -> PathBuf { home_dir().join("models.toml") }

/// The file as read: its TOML, or why it could not be read, and the programs it allows.
#[derive(Clone, Debug, Default)]
pub struct Raw { pub table: toml::Table, pub file: bool, pub allow: Option<Vec<String>>, pub problems: Vec<String> }
static LAST: Mutex<Option<(String, Arc<Raw>)>> = Mutex::new(None);
/// The file's mark on disk: it is read again only when that changes. Your settings' own mark holds it too, since a
/// value there is checked against the programs it allows.
pub fn mark() -> String {
    use std::os::unix::fs::MetadataExt;
    let f = path();
    std::fs::metadata(&f).map(|m| format!("{}|{}|{}|{}.{}|{}.{}", f.display(), m.ino(), m.size(), m.mtime(), m.mtime_nsec(), m.ctime(), m.ctime_nsec())).unwrap_or_else(|_| format!("{}|none", f.display()))
}
pub fn raw() -> Arc<Raw> {
    let m = mark();
    if let Some((k, r)) = LAST.lock().unwrap().as_ref() { if *k == m { return r.clone(); } }
    let r = Arc::new(match std::fs::read_to_string(path()) { Ok(t) => raw_of(&t, true), Err(_) => raw_of(EXAMPLE, false) });
    *LAST.lock().unwrap() = Some((m, r.clone()));
    r
}
/// A catalog's text, read: its table and the programs it allows.
pub fn raw_of(text: &str, file: bool) -> Raw {
    let mut r = Raw { file, ..Default::default() };
    match toml::from_str::<toml::Table>(text) {
        Ok(t) => r.table = t,
        Err(e) => {
            // The programs it allows still hold, read from their own line, so a slip elsewhere in the file never lets
            // every program in again.
            let kept = allow_line(text);
            r.problems.push(format!("models.toml is not valid TOML, so nothing in it counts{}: {}", if kept.is_some() { " but its allow.agents" } else { "" }, one_line(&e.to_string())));
            if let Some(t) = kept { r.table = t; r.table.remove("entry"); } else { return r; }
        }
    }
    for k in r.table.keys() { if !["allow", "entry"].contains(&k.as_str()) { r.problems.push(format!("unknown part \"{k}\": a catalog has allow.agents and [[entry]] tables")); } }
    if let Some(a) = r.table.get("allow") {
        match a.get("agents").map(|v| v.as_array().map(|l| l.iter().map(|x| x.as_str().map(String::from)).collect::<Option<Vec<_>>>())) {
            None => {}
            Some(Some(Some(list))) => {
                let mut ok = vec![];
                for x in list { match crate::agents::kinds::kind_named(&x) { Some(k) => if !ok.contains(&k.name.to_string()) { ok.push(k.name.to_string()) }, None => r.problems.push(format!("allow.agents: no agent program \"{x}\" (it must be {})", crate::agents::kinds::KIND_NAMES.join(", "))) } }
                r.allow = Some(ok);
            }
            _ => r.problems.push("allow.agents must be a list of programs, such as [\"claude\"]: every program is allowed until it is".into()),
        }
    }
    r
}
/// The allow.agents line of a file that is not valid TOML as a whole: `agents = [...]` under `[allow]`, or `allow.agents = [...]`.
fn allow_line(text: &str) -> Option<toml::Table> {
    let mut section = String::new();
    for l in text.lines().map(str::trim) {
        if l.starts_with('[') && !l.starts_with("[[") { section = l.trim_matches(|c| c == '[' || c == ']').trim().to_string(); continue; }
        if l.starts_with("[[") { section = String::new(); continue; }
        let line = if section == "allow" && l.starts_with("agents") { format!("[allow]\n{l}") } else if section.is_empty() && l.starts_with("allow.agents") { l.to_string() } else { continue };
        if let Ok(t) = toml::from_str::<toml::Table>(&line) { return Some(t); }
    }
    None
}
fn one_line(s: &str) -> String { s.split_whitespace().collect::<Vec<_>>().join(" ") }

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Entry { pub name: String, pub agent: String, pub effort: String, pub cost: f64, pub good_for: String, pub avoid_for: String, pub escalate_to: String, pub family: String, pub equivalents: Vec<String>, pub max_files: Option<i64>, pub max_minutes: Option<i64> }
impl Entry {
    pub fn json(&self) -> Value {
        json!({ "name": self.name, "agent": self.agent, "effort": self.effort, "cost": self.cost, "good_for": self.good_for, "avoid_for": self.avoid_for,
            "escalate_to": if self.escalate_to.is_empty() { Value::Null } else { json!(self.escalate_to) }, "family": self.family, "equivalents": self.equivalents, "max_files": self.max_files, "max_minutes": self.max_minutes })
    }
}
#[derive(Clone, Debug, Default)]
pub struct Catalog { pub entries: Vec<Entry>, pub problems: Vec<String>, pub file: bool, pub allow: Option<Vec<String>> }
impl Catalog {
    pub fn get(&self, name: &str) -> Option<&Entry> { self.entries.iter().find(|e| e.name.eq_ignore_ascii_case(name.trim())) }
    pub fn names(&self) -> Vec<&str> { self.entries.iter().map(|e| e.name.as_str()).collect() }
    /// The entry and the ones it escalates to, in order, each once.
    pub fn chain(&self, name: &str) -> Vec<Entry> {
        let mut out: Vec<Entry> = vec![];
        let mut at = self.get(name).cloned();
        while let Some(e) = at { if out.iter().any(|x| x.name == e.name) { break; } at = if e.escalate_to.is_empty() { None } else { self.get(&e.escalate_to).cloned() }; out.push(e); }
        out
    }
}
/// Your catalog, as it reads now.
pub fn load() -> Catalog { from(&raw()) }
/// A catalog's text, read and checked: for the tests, and for a file not yet saved.
pub fn parse(text: &str) -> Catalog { from(&raw_of(text, true)) }

const KEYS: [&str; 11] = ["name", "agent", "effort", "cost", "good_for", "avoid_for", "escalate_to", "family", "equivalents", "max_files", "max_minutes"];
fn entry_of(i: usize, v: &toml::Value, allow: &Option<Vec<String>>) -> Result<Entry, String> {
    let Some(t) = v.as_table() else { return Err(format!("entry {}: not a table", i + 1)) };
    let word = |k: &str| t.get(k).and_then(|x| x.as_str()).map(|s| s.trim().to_string()).unwrap_or_default();
    let name = word("name");
    let who = if name.is_empty() { format!("entry {}", i + 1) } else { format!("entry {name}") };
    if name.is_empty() { return Err(format!("{who}: it needs a name (name = \"leve\")")); }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') { return Err(format!("{who}: a name is one word of letters, digits, - and _")); }
    if let Some(k) = t.keys().find(|k| !KEYS.contains(&k.as_str())) { return Err(format!("{who}: unknown key \"{k}\"{}", crate::store::settings::values::nearest(k, &KEYS).map(|n| format!(" (did you mean {n}?)")).unwrap_or_default())); }
    for k in ["agent", "effort", "good_for", "avoid_for", "escalate_to", "family"] { if t.get(k).map(|x| !x.is_str()).unwrap_or(false) { return Err(format!("{who}: {k} is words, in quotes")); } }
    let equivalents = super::equivalents::read(t.get("equivalents")).map_err(|e| format!("{who}: {e}"))?;
    if word("agent").is_empty() { return Err(format!("{who}: it needs an agent, a program with an optional model after a slash (agent = \"claude/sonnet\")")); }
    let agent = text(&check("worker.agent", &json!(word("agent")), "").map_err(|e| format!("{who}: {}", e.msg.trim_start_matches("worker.agent").trim_start_matches(": ")))?);
    let kind = crate::store::settings::values::agent_of(&agent).kind;
    if let Some(l) = allow { if !l.contains(&kind) { return Err(format!("{who}: {kind} is not allowed here: allow.agents allows only {}", l.join(", "))); } }
    let effort = if word("effort").is_empty() { "default".to_string() } else { text(&check("worker.effort", &json!(word("effort")), "").map_err(|e| format!("{who}: {}", e.msg))?) };
    let why = fits("worker.effort", &effort, &agent, false);
    if !why.is_empty() { return Err(format!("{who}: effort {effort}: {why}")); }
    let num = |k: &str| t.get(k).map(|x| x.as_float().or_else(|| x.as_integer().map(|n| n as f64)).filter(|n| *n >= 0.0).ok_or(format!("{who}: {k} is a number, 0 or more")));
    let cost = num("cost").transpose()?.unwrap_or(1.0);
    let whole = |k: &str| -> Result<Option<i64>, String> { t.get(k).map(|x| x.as_integer().filter(|n| *n > 0).ok_or(format!("{who}: {k} is a whole number, 1 or more"))).transpose() };
    Ok(Entry { name, agent, effort, cost, good_for: word("good_for"), avoid_for: word("avoid_for"), escalate_to: word("escalate_to"), family: word("family"), equivalents, max_files: whole("max_files")?, max_minutes: whole("max_minutes")? })
}
fn from(r: &Raw) -> Catalog {
    let mut c = Catalog { file: r.file, problems: r.problems.clone(), allow: r.allow.clone(), ..Default::default() };
    let list: Vec<toml::Value> = match r.table.get("entry") { None => vec![], Some(toml::Value::Array(a)) => a.clone(), Some(_) => { c.problems.push("entry must be [[entry]] tables".into()); vec![] } };
    for (i, v) in list.iter().enumerate() {
        match entry_of(i, v, &r.allow) {
            Ok(e) if c.get(&e.name).is_some() => c.problems.push(format!("entry {}: the name is taken by an earlier entry, so this one is not used", e.name)),
            Ok(e) => c.entries.push(e),
            Err(why) => c.problems.push(format!("{}; this entry is not used", why.trim_end_matches('.'))),
        }
    }
    // An escalation goes to an entry that is there, and never round in a circle.
    let names: Vec<String> = c.entries.iter().map(|e| e.name.clone()).collect();
    for i in 0..c.entries.len() {
        let to = c.entries[i].escalate_to.clone();
        if to.is_empty() { continue; }
        let gone = !names.iter().any(|n| n.eq_ignore_ascii_case(&to));
        let circle = !gone && c.chain(&to).iter().any(|e| e.name == c.entries[i].name);
        if gone || circle {
            c.problems.push(format!("entry {}: escalate_to \"{to}\" {}, so it escalates to nothing", c.entries[i].name, if gone { "is no entry of the catalog" } else { "goes round in a circle" }));
            c.entries[i].escalate_to.clear();
        } else if let Some(n) = names.iter().find(|n| n.eq_ignore_ascii_case(&to)) { c.entries[i].escalate_to = n.clone(); }
    }
    super::equivalents::validate(&mut c);
    c
}
