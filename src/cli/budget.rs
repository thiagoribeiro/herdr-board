//! Shared budget status and explicit human reconciliation; inspection never starts scheduled model work.
use super::flags::Args;
use crate::{err::Res, json::*};
pub fn command(a: &Args) -> Res<(Value, String)> {
    match a.pos.get(1).map(String::as_str).unwrap_or("status") {
        "status" => {},
        "settle" => {
            let owner = a.pos.get(2).ok_or("budget settle needs an owner from budget status")?;
            let used = a.flags.so("used").ok_or("budget settle needs --used TOKENS, the owner's reconciled cumulative use")?;
            let checked = if used == "0" { json!("0") } else { crate::store::settings::values::check("limit.tokens", &json!(used), "")? };
            if checked == json!("none") { crate::fail!("Reconcile an explicit token count, not none"); }
            let total = crate::store::settings::values::tokens_of(&text(&checked)) as u64;
            crate::budget::settle_confirmed(owner, total, a.flags.b("ended"))?;
        }
        _ => crate::fail!("budget takes status or settle OWNER --used TOKENS"),
    }
    let value = crate::budget::status()?;
    Ok((value.clone(), serde_json::to_string_pretty(&value)?))
}
