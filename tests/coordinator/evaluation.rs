//! Evaluation records keep independent criteria, failures, missing usage and paired trials visible without calling models.
use crate::common;
use common::*;
use sw::{evaluation::{self, cases::CASES}, json::*};

fn assessment(case: &str, variant: &str, trial: i64) -> Value {
    let c = evaluation::cases::find(case).unwrap();
    json!({ "case": case, "variant": variant, "input": "same-initial-commit-and-request", "trial": trial,
        "assessedBy": "test evaluator", "elapsedMs": 1000, "interruptions": 1, "escapedDefects": 0, "unauthorizedActions": 0,
        "criteria": c.criteria.iter().map(|(k, _)| ((*k).into(), json!(true))).collect::<Obj>(),
        "evidence": c.criteria.iter().map(|(k, _)| ((*k).into(), json!("Independent check result in the test fixture"))).collect::<Obj>() })
}
fn task_with_tokens(tokens: i64) -> Obj {
    obj(json!({ "id": "proj/1", "stage": "done", "spent": { "total": tokens, "partial": false,
        "metrics": { "input": tokens }, "files": { "conversation": { "n": tokens } } } }))
}
fn record(case: &str, variant: &str, trial: i64, tokens: i64) -> Value {
    evaluation::record(&task_with_tokens(tokens), &assessment(case, variant, trial)).unwrap()
}

#[test]
fn assessments_require_every_criterion_original_evidence_and_an_ended_execution() {
    let _w = world(); let t = task_with_tokens(100);
    let mut a = assessment("edge-fix", "baseline", 1);
    assert!(evaluation::record(&t, &a).unwrap().b("success"));
    a["evidence"]["fixed"] = json!("");
    assert!(evaluation::record(&t, &a).is_err());
    a = assessment("edge-fix", "baseline", 1); a["criteria"].as_object_mut().unwrap().remove("scoped");
    assert!(evaluation::record(&t, &a).is_err());
    let mut working = t.clone(); working.put("stage", "working");
    assert!(evaluation::record(&working, &assessment("edge-fix", "baseline", 1)).is_err());
    a = assessment("edge-fix", "baseline", 1); a["interruptions"] = json!(-1);
    assert!(evaluation::record(&t, &a).is_err());
    a = assessment("edge-fix", "baseline", 1); a["assessedBy"] = json!("");
    assert!(evaluation::record(&t, &a).is_err());
}

#[test]
fn a_failed_check_or_review_cannot_be_hidden_by_a_successful_assessment() {
    let _w = world(); let mut t = task_with_tokens(100);
    t.put("failure", json!({ "title": "Independent check failed" }));
    let r = evaluation::record(&t, &assessment("edge-fix", "candidate", 1)).unwrap();
    assert!(!r.b("success"));
    t.del("failure"); t.put("review", json!({ "meets": false, "findings": [{ "severity": "major" }], "pass": false }));
    assert!(!evaluation::record(&t, &assessment("edge-fix", "candidate", 1)).unwrap().b("success"));
    t.del("review"); t.put("prFailure", json!({ "title": "CI failed" }));
    let r = evaluation::record(&t, &assessment("edge-fix", "candidate", 1)).unwrap();
    assert!(!r.b("success"));
    let baseline = record("edge-fix", "baseline", 1, 200);
    assert!(!evaluation::compare(&[baseline], &[r]).unwrap().b("qualityPreserved"));
}

#[test]
fn lower_usage_does_not_hide_regressions_and_incomplete_usage_never_becomes_zero() {
    let _w = world();
    let baseline = record("edge-fix", "baseline", 1, 1000);
    let mut a = assessment("edge-fix", "candidate", 1); a["criteria"]["fixed"] = json!(false);
    let candidate = evaluation::record(&task_with_tokens(100), &a).unwrap();
    let compared = evaluation::compare(&[baseline.clone()], &[candidate]).unwrap();
    assert!(!compared.b("qualityPreserved") && !compared.b("readyForTaskUsageDecision"));
    assert_eq!(compared.arr("regressions").len(), 1);
    let mut t = task_with_tokens(100); t.obj_mut("spent").put("partial", true);
    let candidate = evaluation::record(&t, &assessment("edge-fix", "candidate", 1)).unwrap();
    assert!(candidate.at("usage").at("tokens").is_null());
    let compared = evaluation::compare(&[baseline], &[candidate]).unwrap();
    assert!(compared.at("taskTokens").is_null() && compared.at("tokenReduction").is_null());
}

#[test]
fn comparisons_require_identical_trials_and_reject_duplicates_or_tampered_success_flags() {
    let _w = world(); let a = record("edge-fix", "baseline", 1, 1000);
    let mut b = record("edge-fix", "candidate", 1, 500);
    assert!(evaluation::compare(&[a.clone(), a.clone()], &[b.clone()]).is_err());
    assert!(evaluation::compare(&[a.clone()], &[record("edge-fix", "candidate", 2, 500)]).is_err());
    b["assessment"]["unauthorizedActions"] = json!(1);
    assert!(evaluation::compare(&[a], &[b]).is_err(), "success must agree with the actual assessment");
}

#[test]
fn full_repeated_suite_and_preserved_quality_are_required_before_a_usage_decision() {
    let _w = world();
    let trials = |variant, tokens| CASES.iter().flat_map(|c| (1..=3).map(move |trial| record(c.id, variant, trial, tokens))).collect::<Vec<_>>();
    let a = trials("baseline", 1000); let b = trials("candidate", 800);
    let compared = evaluation::compare(&a, &b).unwrap();
    assert!(compared.b("suiteComplete") && compared.b("qualityPreserved") && compared.b("readyForTaskUsageDecision"));
    assert_eq!(compared.i("pairs"), 24);
    assert!((compared.f("tokenReduction") - 0.2).abs() < 0.00001);
    assert!(compared.s("scope").contains("overhead requires separate measurement"));
    assert!(!evaluation::compare(&a[..1], &b[..1]).unwrap().b("readyForTaskUsageDecision"));
}

#[test]
fn evaluation_cli_lists_cases_and_records_a_task_without_starting_a_model_or_approving_it() {
    let w = world(); let id = w.add("Real trial to assess");
    sw::store::tasks::patch(&id, |t| t.put("stage", "approval")).unwrap();
    let file = w.base.join("assessment.json");
    std::fs::write(&file, assessment("edge-fix", "baseline", 1).to_string()).unwrap();
    let run = |words: &[&str]| {
        let a = sw::cli::flags::parse(&words.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        sw::cli::flags::refuse_unknown("evaluate", &a).unwrap(); sw::cli::tasks::command("evaluate", &a).unwrap().0
    };
    assert_eq!(run(&["evaluate", "cases"]).arr("cases").len(), 8);
    assert_eq!(sw::cli::main(vec!["evaluate".into(), "cases".into(), "--json".into()]), 0);
    let result = run(&["evaluate", "record", &id, "--assessment", &file.display().to_string()]);
    assert_eq!(result.s("task"), id);
    assert!(result.at("usage").b("partial"));
    assert_eq!(task(&id).s("stage"), "approval");
    assert!(sw::runner::JOBS.with(|jobs| jobs.borrow().is_empty()));
    assert!(w.state().arr("panes").is_empty());
}
