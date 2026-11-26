//! Opt-in evaluation cases, evidence-backed trial records and paired comparisons; no agent starts and no trial is saved implicitly.
use super::flags::Args;
use crate::{err::Res, evaluation, json::*};

fn read(path: &str) -> Res<Value> { Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?) }
fn trials(path: &str) -> Res<Vec<Value>> {
    let raw = std::fs::read_to_string(path)?;
    if let Ok(Value::Array(rows)) = serde_json::from_str::<Value>(&raw) { return Ok(rows); }
    raw.lines().filter(|l| !l.trim().is_empty()).map(|l| serde_json::from_str(l).map_err(Into::into)).collect()
}
pub fn command(a: &Args) -> Res<(Value, String)> {
    let need = |n: usize| a.pos.get(n).filter(|s| !s.is_empty()).map(String::as_str).ok_or_else(|| crate::err::Error::from("Use evaluate cases, record ID --assessment FILE, or compare BASELINE CANDIDATE"));
    let v = match a.pos.get(1).map(String::as_str).unwrap_or("cases") {
        "cases" => evaluation::cases::list(),
        "record" => {
            let t = crate::store::tasks::find(need(2)?)?;
            let file = a.flags.so("assessment").ok_or_else(|| crate::err::Error::from("record needs --assessment FILE"))?;
            let assessment = read(file)?;
            // Validate the human assessment before refreshing usage, which writes only accounting evidence.
            evaluation::record(&t, &assessment)?;
            let t = crate::limits::spent::count(t.s("id"), "agent", 2000).unwrap_or(t);
            evaluation::record(&t, &assessment)?
        }
        "compare" => evaluation::compare(&trials(need(2)?)?, &trials(need(3)?)?)?,
        _ => crate::fail!("evaluate takes cases, record ID --assessment FILE, or compare BASELINE CANDIDATE"),
    };
    Ok((v.clone(), serde_json::to_string_pretty(&v)?))
}
