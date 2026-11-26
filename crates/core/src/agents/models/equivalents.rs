//! Declared model correspondences within one capability family; availability alternatives are separate from quality escalation.
use super::catalog::{Catalog, Entry};
pub fn read(value: Option<&toml::Value>) -> Result<Vec<String>, String> {
    let Some(value) = value else { return Ok(vec![]) };
    let Some(list) = value.as_array() else { return Err("equivalents must be an ordered list of entry names".into()) };
    let mut out = vec![];
    for item in list {
        let Some(name) = item.as_str().map(str::trim).filter(|s| !s.is_empty()) else { return Err("equivalents must contain nonempty entry names".into()) };
        if !out.iter().any(|n: &String| n.eq_ignore_ascii_case(name)) { out.push(name.to_string()); }
    }
    Ok(out)
}
pub fn validate(c: &mut Catalog) {
    let entries = c.entries.clone();
    for entry in &mut c.entries {
        let mut keep = vec![];
        for name in &entry.equivalents {
            let target = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name));
            let why = match target {
                None => "is no usable catalog entry",
                Some(e) if e.name == entry.name || e.agent == entry.agent => "names the same model",
                Some(e) if crate::store::settings::values::agent_of(&entry.agent).model.is_empty() || crate::store::settings::values::agent_of(&e.agent).model.is_empty() => "requires explicit program/model on both entries",
                Some(e) if entry.family.is_empty() || e.family.is_empty() || e.family != entry.family => "does not belong to the same explicit family",
                Some(_) => "",
            };
            if why.is_empty() { keep.push(target.unwrap().name.clone()); }
            else { c.problems.push(format!("entry {}: equivalent \"{name}\" {why}; it will not be used", entry.name)); }
        }
        entry.equivalents = keep;
    }
}
/// An entry name is unambiguous. An agent without a model, or with several catalog profiles, is never guessed.
pub fn find<'a>(c: &'a Catalog, name: &str) -> Option<&'a Entry> {
    if let Some(e) = c.get(name) { return Some(e); }
    if crate::store::settings::values::agent_of(name).model.is_empty() { return None; }
    let mut found = c.entries.iter().filter(|e| e.agent == name);
    let entry = found.next()?;
    found.next().is_none().then_some(entry)
}
pub fn candidates(c: &Catalog, name: &str) -> Vec<Entry> {
    let Some(e) = find(c, name) else { return vec![] };
    e.equivalents.iter().filter_map(|n| c.get(n).cloned()).collect()
}
/// Only infrastructure/availability failures opt into these peers; content refusals and context limits do not.
pub fn unavailable(error: &str, passing: bool, auth: bool) -> bool {
    if error.is_empty() || crate::re!(r"(?i)safety|policy|refus(?:e|al)|content.filter|not.allowed").is_match(error) { return false; }
    passing || auth || crate::re!(r"(?i)(?:usage|session).limit|out of (?:credits?|allowance)|\bquota\b|billing|credit balance|payment|model[^\n]{0,80}(?:not found|not supported|unsupported|does not exist|unavailable)|unsupported.model|overload|capacity|service.unavailable|connection|network|timed?.out").is_match(error)
}
