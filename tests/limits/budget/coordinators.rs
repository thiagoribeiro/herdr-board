//! Coordinators and native workers share one allowance, and conversation checkpoints never create a new token balance.
use crate::common;
use common::{*, pilot::*};
use sw::{budget, coordinator::{co::Co, state}, json::*};

#[test]
fn project_coordinators_and_workers_reserve_from_the_same_balance() {
    let w = pilot_world_with(json!({ "budget.tokens": "1k", "budget.reserve": "700", "budget.reviewPercent": 0 }));
    w.script(json!({ "coordinator": [{ "status": "working" }, { "status": "working" }], "worker": [{ "status": "working" }] }));
    ok(&["coordinator", "tell", "Plan the task", "--project", "proj"]); drain();
    let co = proj(&w);
    let coord = state::get(&co).unwrap();
    assert!(coord.b("mark"), "the first budgeted coordinator prompt was admitted: {coord:?}");
    let id = w.add("Work with the remaining allowance"); delegate(&id); assert_eq!(settle(&id).s("stage"), "working");
    let s = budget::status().unwrap();
    assert_eq!(s.at("owners").at(&budget::admission::coordinator_owner(&co)).i("held"), 700);
    assert_eq!(s.at("owners").at(&format!("task:{id}")).i("held"), 300);
    ok(&["coordinator", "tell", "Another coordinator", "--machine"]); drain();
    let machine = state::get(&Co::machine(&w.socket)).unwrap();
    assert!(!machine.b("mark"));
    assert!(machine.s("error").contains("budget"));
    let session = state::get(&co).t("session");
    let file = std::path::PathBuf::from(std::env::var("CLAUDE_CONFIG_DIR").unwrap()).join(format!("projects/x/{session}.jsonl"));
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, format!("{}\n", json!({ "type": "assistant", "message": { "id": "coord-reply", "usage": { "input_tokens": 1100, "output_tokens": 10 } } }))).unwrap();
    sw::clock::advance(3000);
    budget::runtime::tick(&sw::flow::live::board_for(&w.socket), &w.socket);
    assert!(state::get(&co).b("budgetStop"));
    assert_eq!(budget::status().unwrap().at("global").i("used"), 1110);
}

#[test]
fn a_checkpoint_does_not_make_missing_earlier_usage_a_fresh_zero_cost_session() {
    let w = pilot_world_with(json!({ "budget.tokens": "1k", "budget.reserve": "700", "budget.reviewPercent": 0 }));
    let co = proj(&w);
    state::change(&co, |c| { c.put("run", json!({ "kind": "claude" })); c.put("session", "not-yet-saved"); c.put("checkpoints", json!([{ "session": "old" }])); true }).unwrap();
    assert!(budget::admission::coordinator(&co).is_err());
    let s = budget::status().unwrap();
    assert!(s.at("owners").at(&budget::admission::coordinator_owner(&co)).b("uncertain"));
}
