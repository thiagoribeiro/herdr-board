//! Shared budget reservations across concurrent owners, restarts, project caps and uncertain provider outcomes.
use crate::common;
use common::*;
use sw::{budget, json::*};
mod runtime;
mod coordinators;
mod evidence;
fn settings() -> Value { json!({ "budget.tokens": "1k", "budget.reserve": "700", "budget.reviewPercent": 0, "budget.period": "1h" }) }
fn world_budget() -> World { world_with(Opts { repo: true, settings: settings(), ..Default::default() }) }

#[test]
fn concurrent_coordinators_and_tasks_cannot_reserve_the_same_allowance() {
    let _w = world_budget();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let handles: Vec<_> = ["coordinator:one", "task:proj/1"].into_iter().map(|id| {
        let barrier = barrier.clone();
        std::thread::spawn(move || { barrier.wait(); budget::acquire(id, "proj", "agent", Some(0)).unwrap().i("reserved") })
    }).collect();
    let total: i64 = handles.into_iter().map(|h| h.join().unwrap()).sum();
    assert_eq!(total, 1000);
    assert!(budget::acquire("task:proj/2", "proj", "agent", Some(0)).is_err());
    assert_eq!(budget::status().unwrap().at("global").i("reserved"), 1000);
}

#[test]
fn measurements_are_incremental_repeated_claims_are_idempotent_and_unknown_results_keep_their_hold() {
    let _w = world_budget();
    budget::acquire("task:proj/1", "proj", "agent", Some(100)).unwrap();
    budget::observe("task:proj/1", "proj", Some(140), false).unwrap();
    budget::observe("task:proj/1", "proj", Some(140), false).unwrap();
    assert_eq!(budget::acquire("task:proj/1", "proj", "agent", Some(140)).unwrap().i("reserved"), 660);
    assert_eq!(budget::status().unwrap().at("global").i("used"), 40);
    assert!(budget::settle("task:proj/1", 150).is_err(), "a running owner cannot be reconciled away");
    budget::observe("task:proj/1", "proj", None, true).unwrap();
    let stopped = budget::status().unwrap();
    assert_eq!(stopped.at("global").i("reserved"), 660);
    assert!(stopped.at("owners").at("task:proj/1").b("uncertain"));
    assert!(budget::acquire("other", "proj", "agent", Some(0)).is_err());
    budget::settle("task:proj/1", 160).unwrap();
    let reconciled = budget::status().unwrap();
    assert_eq!((reconciled.at("global").i("used"), reconciled.at("global").i("reserved")), (60, 0));
    assert!(budget::acquire("other", "proj", "agent", Some(0)).is_ok());
    assert!(budget::settle("task:proj/1", 0).is_err(), "manual reconciliation never refunds observed tokens");
}

#[test]
fn period_rollover_preserves_live_reservations_and_a_clock_rollback_grants_nothing() {
    let _w = world_budget(); let at = sw::clock::now(); sw::clock::set(Some(at));
    budget::acquire("task", "proj", "agent", Some(0)).unwrap();
    budget::observe("task", "proj", Some(100), false).unwrap();
    sw::clock::advance(3600000);
    let next = budget::status().unwrap();
    assert_eq!((next.at("global").i("used"), next.at("global").i("reserved")), (0, 600));
    budget::observe("task", "proj", Some(150), false).unwrap();
    assert_eq!(budget::status().unwrap().at("global").i("used"), 50);
    sw::clock::set(Some(at));
    let back = budget::status().unwrap();
    assert_eq!(back.at("window"), next.at("window"));
    assert_eq!(back.at("global").i("used"), 50);
}

#[test]
fn workers_leave_a_review_allowance_and_review_uses_the_same_task_account() {
    let _w = world_with(Opts { repo: true, settings: json!({ "budget.tokens": "1k", "budget.reserve": "1k", "budget.reviewPercent": 20 }), ..Default::default() });
    assert_eq!(budget::acquire("task", "proj", "agent", Some(0)).unwrap().i("reserved"), 800);
    budget::observe("task", "proj", Some(800), true).unwrap();
    assert!(budget::acquire("another-worker", "proj", "agent", Some(0)).is_err());
    assert_eq!(budget::acquire("task", "proj", "reviewer", Some(800)).unwrap().i("reserved"), 200);
    budget::observe("task", "proj", Some(950), true).unwrap();
    assert_eq!(budget::status().unwrap().at("global").i("used"), 950);
}

#[test]
fn project_caps_share_across_owners_and_cannot_be_moved_or_set_by_task_code() {
    let w = world_with(Opts { repo: true, settings: json!({ "budget.reserve": "700", "budget.reviewPercent": 0,
        "repos": { "proj": { "budget.projectTokens": "1k" } } }), ..Default::default() });
    let id = w.add("Scoped task");
    assert_eq!(budget::acquire("coordinator", "proj", "coordinator", Some(0)).unwrap().i("reserved"), 700);
    assert_eq!(budget::acquire("task", "proj", "agent", Some(0)).unwrap().i("reserved"), 300);
    assert!(budget::acquire("third", "proj", "agent", Some(0)).is_err());
    assert!(budget::acquire("coordinator", "another-project", "coordinator", Some(0)).is_err());
    assert!(sw::store::settings::change::set_value("budget.projectTokens", &json!("none"), &sw::store::settings::lookup::Scope::task(&task(&id))).is_err());
    std::fs::write(w.root.join(".herdr-board.json"), r#"{"budget.projectTokens":"999m","budget.tokens":"999m"}"#).unwrap();
    assert_eq!(budget::config::project("proj"), Some(1000));
}

#[test]
fn corrupt_state_or_invalid_settings_are_preserved_and_never_reset_to_an_unlimited_budget() {
    let _w = world_budget(); budget::acquire("task", "proj", "agent", Some(0)).unwrap();
    let ledger = sw::store::files::data_dir().join("budget-ledger.json");
    std::fs::write(&ledger, "broken").unwrap();
    assert!(budget::acquire("other", "proj", "agent", Some(0)).is_err());
    assert_eq!(std::fs::read_to_string(&ledger).unwrap(), "broken");
    std::fs::write(sw::store::settings::places::config_file(), r#"{"budget.tokens":"invalid"}"#).unwrap();
    assert!(budget::tracking());
    assert!(budget::status().is_err());
}
