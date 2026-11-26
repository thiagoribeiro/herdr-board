//! Declared same-family equivalents, ambiguity handling, profile inspection and separation from quality escalation.
use crate::common;
use common::*;
use sw::{agents::models::{catalog, equivalents}, json::*};
pub const PAIRS: &str = r#"
[[entry]]
name = "routine"
agent = "claude/sonnet"
effort = "high"
family = "routine"
equivalents = ["routine_codex"]
escalate_to = "complex"
[[entry]]
name = "routine_codex"
agent = "codex/gpt-6.1-sol"
effort = "medium"
family = "routine"
equivalents = ["routine"]
[[entry]]
name = "complex"
agent = "claude/opus"
effort = "high"
family = "complex"
"#;
pub fn pairs() { std::fs::write(catalog::path(), PAIRS).unwrap(); }

#[test]
fn reciprocal_peers_are_valid_without_turning_into_quality_escalation_cycles() {
    let _w = world(); let c = catalog::parse(PAIRS);
    assert!(c.problems.is_empty(), "{:?}", c.problems);
    assert_eq!(equivalents::candidates(&c, "routine")[0].agent, "codex/gpt-6.1-sol");
    assert_eq!(equivalents::candidates(&c, "routine_codex")[0].agent, "claude/sonnet");
    assert_eq!(c.chain("routine").iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), vec!["routine", "complex"]);
    assert!(equivalents::candidates(&c, "claude").is_empty(), "the unspecified default has no inferred quality");
    let duplicate = catalog::parse(&format!("{PAIRS}\n[[entry]]\nname='other'\nagent='claude/sonnet'\n"));
    assert!(equivalents::find(&duplicate, "claude/sonnet").is_none());
}

#[test]
fn malformed_cross_family_missing_and_self_edges_are_not_eligible() {
    let _w = world();
    let c = catalog::parse(&PAIRS.replace("[\"routine_codex\"]", "[\"complex\", \"routine\", \"missing\"]"));
    assert!(c.get("routine").unwrap().equivalents.is_empty());
    assert_eq!(c.problems.len(), 3, "{:?}", c.problems);
    assert!(catalog::parse(&PAIRS.replace("[\"routine_codex\"]", "\"routine_codex\"")).get("routine").is_none());
    assert!(catalog::parse(&PAIRS.replace("[\"routine_codex\"]", "[1]")).get("routine").is_none());
    assert!(catalog::parse(&PAIRS.replace("codex/gpt-6.1-sol", "codex")).get("routine").unwrap().equivalents.is_empty());
    assert!(!equivalents::unavailable("The model refused to continue under its safety policy", false, false));
    assert!(!equivalents::unavailable("Model is not supported by organization policy", true, false));
    assert!(equivalents::unavailable("Usage limit reached", false, false));
    assert!(equivalents::unavailable("You've hit your session limit · resets 5pm", false, false));
    assert!(equivalents::unavailable("401 Unauthorized", false, true));
    assert!(!equivalents::unavailable("context window exceeded", false, false));
}

#[test]
fn native_model_unavailability_errors_use_the_same_classification_as_the_failure_report() {
    for error in ["model_not_found", "Unknown model sonnet", "Model sonnet is not available", "Model sonnet doesn't exist"] {
        assert!(sw::agents::model_refused::read("claude", error).is_some(), "{error}");
        assert!(equivalents::unavailable(error, false, false), "{error}");
    }
    assert!(!equivalents::unavailable("Model is not allowed by content policy", false, false));
}

#[test]
fn counterparts_only_handle_availability_and_explicit_none_keeps_them_off() {
    let w = world_with(Opts { repo: true, settings: json!({ "fallback.corresponding": true }), ..Default::default() }); pairs();
    let id = w.add("Task");
    sw::store::route::choose(&id, "routine", "", "coordinator", &[]).unwrap();
    let t = task(&id); let settings = sw::store::settings::lookup::work(Some(&t));
    assert_eq!(sw::store::corresponding::choices(&t, "worker", "claude/sonnet", &settings, true).agents, vec!["codex/gpt-6.1-sol"]);
    assert_eq!(sw::store::corresponding::choices(&t, "worker", "claude/sonnet", &settings, false).agents, vec!["claude/opus"]);
    sw::store::settings::change::set_value("worker.fallback", &json!("none"), &sw::store::settings::lookup::Scope::task(&t)).unwrap();
    let t = task(&id); let settings = sw::store::settings::lookup::work(Some(&t));
    assert!(sw::store::corresponding::choices(&t, "worker", "claude/sonnet", &settings, true).agents.is_empty());
}

#[test]
fn the_correspondence_profile_is_optional_and_existing_catalogs_cannot_be_overwritten() {
    let _w = world();
    let profile = include_str!("../../crates/core/src/agents/models/models.corresponding.toml");
    let c = catalog::parse(profile); assert!(c.problems.is_empty(), "{:?}", c.problems);
    assert_eq!(c.entries.len(), 7);
    sw::agents::models::catalog_say::init_profile("corresponding").unwrap();
    assert_eq!(read(&catalog::path()), profile);
    assert!(sw::agents::models::catalog_say::init_profile("default").is_err());
    pairs();
    let (j, text) = sw::agents::models::equivalents_say::command("routine").unwrap();
    assert!(!j.b("validatedQuality")); assert!(text.contains("routine_codex"));
    assert!(sw::agents::models::equivalents_say::command("claude").is_err());
}

#[test]
fn catalog_import_validates_before_replacing_and_keeps_the_previous_custom_catalog() {
    let w = world(); pairs();
    let input = w.base.join("catalog.toml");
    std::fs::write(&input, "[[entry]]\nname='broken'\nagent='invalid'\n").unwrap();
    assert!(sw::agents::models::catalog_say::import(&input).is_err());
    assert_eq!(read(&catalog::path()), PAIRS);
    let changed = PAIRS.replace("effort = \"high\"", "effort = \"medium\"");
    std::fs::write(&input, &changed).unwrap();
    sw::agents::models::catalog_say::import(&input).unwrap();
    assert_eq!(read(&catalog::path()), changed);
    assert_eq!(read(&catalog::path().with_extension("previous.toml")), PAIRS);
}
