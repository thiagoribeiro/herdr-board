//! The tests of values.rs: a typed value as the board keeps it, or refused with the nearest right one.
use super::*;
#[test]
fn values() {
    assert_eq!(check("review.rounds", &json!("5"), "").unwrap(), json!(5));
    assert_eq!(check("limit.tokens", &json!("2000k"), "").unwrap(), json!("2m"));
    assert_eq!(check("limit.time", &json!("none"), "").unwrap(), json!("none"));
    assert!(check("limit.time", &json!("2 hours"), "").unwrap_err().msg.contains("Did you mean 2h?"));
    assert_eq!(check("worker.fallback", &json!("codex, claude/claude-opus-5-5"), "").unwrap(), json!(["codex", "claude/claude-opus-5-5"]));
    assert_eq!(check("attempts.by", &json!("claude, codex"), "").unwrap(), json!("claude, codex"));
    assert!(check("revew.rounds", &json!(1), "").unwrap_err().msg.contains("review.rounds"));
    assert_eq!(tokens_say(1_500_000.0), "1.5m");
    assert_eq!(tokens_say(500_000.0), "500k");
    assert_eq!(check("repo.key", &json!("hsw"), "").unwrap(), json!("HSW"));
    assert!(check("worker.access", &json!("ask"), "pi").is_err());
}
