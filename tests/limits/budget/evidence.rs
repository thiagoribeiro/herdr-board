//! Shared evidence counts once when an attempt's conversation is adopted, copied or polled by another owner.
use crate::common;
use sw::{budget, json::*};
fn files(id: &str, n: u64) -> Value { json!({ "conversation.jsonl": { "kind": "claude", "n": n, "seen": { id: { "input": n } } } }) }

#[test]
fn adopting_a_conversation_does_not_bill_it_twice_and_subsequent_usage_is_charged() {
    let _w = super::world_budget();
    budget::acquire("attempt", "proj", "agent", Some(0)).unwrap();
    budget::observe_measured("attempt", "proj", Some(200), true, &files("reply", 200)).unwrap();
    budget::acquire("parent", "proj", "reviewer", Some(0)).unwrap();
    budget::observe_measured("parent", "proj", Some(200), true, &files("reply", 200)).unwrap();
    assert_eq!(budget::status().unwrap().at("global").i("used"), 200);
    budget::acquire_measured("parent", "proj", "agent", Some(200), &files("reply", 200)).unwrap();
    budget::observe_measured("parent", "proj", Some(250), true, &files("reply", 250)).unwrap();
    // A late poll of the old attempt still sees the shared transcript and cannot charge the new reply again.
    budget::observe_measured("attempt", "proj", Some(250), true, &files("reply", 250)).unwrap();
    assert_eq!(budget::status().unwrap().at("global").i("used"), 250);
}

#[test]
fn enrollment_and_manual_reconciliation_do_not_rebill_historical_evidence() {
    let _w = super::world_budget();
    budget::acquire_measured("task", "proj", "agent", Some(100), &files("reply", 100)).unwrap();
    assert_eq!(budget::status().unwrap().at("global").i("used"), 0);
    budget::observe_measured("task", "proj", Some(120), false, &files("reply", 120)).unwrap();
    budget::observe("task", "proj", None, true).unwrap();
    budget::settle("task", 150).unwrap();
    budget::observe("task", "proj", None, true).unwrap();
    assert!(!budget::status().unwrap().at("owners").at("task").b("uncertain"), "an old missing transcript cannot undo a human reconciliation");
    budget::observe_measured("task", "proj", Some(160), true, &files("reply", 160)).unwrap();
    assert_eq!(budget::status().unwrap().at("global").i("used"), 60);
}

#[test]
fn lost_helper_callbacks_need_an_explicit_human_confirmation_of_the_end() {
    let _w = common::world_with(common::Opts { settings: json!({ "budget.tokens": "1k" }), ..Default::default() });
    budget::acquire("helper:lost", "", "helper", Some(0)).unwrap();
    assert!(budget::settle("helper:lost", 100).is_err());
    budget::settle_confirmed("helper:lost", 100, true).unwrap();
    let s = budget::status().unwrap();
    assert_eq!((s.at("global").i("used"), s.at("global").i("reserved")), (100, 0));
    assert!(s.at("owners").at("helper:lost").b("humanConfirmedEnd"));
}
