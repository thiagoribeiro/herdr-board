//! Report content and its revision, read only within the task's own report folder; large/binary reports remain references.
use crate::json::*;
use std::{io::Read, path::{Component, Path}};
pub fn references(t: &Obj) -> Value {
    json!({ "task": t.at("id"), "head": t.at("prHead"), "reviewHead": crate::store::reviews::current_review(t).map(|r| r.at("head")), "check": t.at("checkPassed"), "report": t.at("report") })
}
pub fn report(t: &Obj) -> Value {
    let name = t.at("report").s("file");
    if name.is_empty() { return Value::Null; }
    if Path::new(name).components().any(|c| !matches!(c, Component::Normal(_))) { return json!({ "unavailable": "invalid report path" }); }
    let base = crate::research::report_folder(t);
    let Ok(base) = std::fs::canonicalize(base) else { return json!({ "unavailable": "report folder missing" }) };
    let Ok(path) = std::fs::canonicalize(base.join(name)) else { return json!({ "unavailable": "report missing" }) };
    if !path.starts_with(&base) { return json!({ "unavailable": "report outside its folder" }); }
    let Ok(mut file) = std::fs::File::open(&path) else { return json!({ "unavailable": "report unreadable" }) };
    let mut hash = sha1_smol::Sha1::new();
    let mut buf = [0u8; 8192]; let mut content = Vec::new(); let mut size = 0;
    loop {
        let n = match file.read(&mut buf) { Ok(0) => break, Ok(n) => n, Err(_) => return json!({ "unavailable": "report read failed" }) };
        hash.update(&buf[..n]); size += n;
        if size <= 24000 { content.extend_from_slice(&buf[..n]); }
    }
    json!({ "hash": hash.digest().to_string(), "bytes": size,
        "text": if size <= 24000 { String::from_utf8(content).ok() } else { None },
        "contentAvailable": size <= 24000 })
}
