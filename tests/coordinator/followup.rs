//! Follow-up work retains the previous scope and evidence; research results reach chat without a coordinator turn.
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::cli::{flags::parse, tasks::command};
use sw::coordinator::co::Co;
use sw::json::*;

fn run(a: &[&str]) -> (Value, String) { command(a[0], &parse(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())).unwrap_or_else(|e| panic!("{}", e.msg)) }

#[test]
fn a_follow_up_is_handed_the_task_it_continues() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Added the parser." }, step(2)] }));
    let first = to_approval(&w, "Add the parser");
    let branch = task(&first).t("branch");
    let (_, said) = run(&["add", "Review the parser", "--research", "--follow-up", &short(&first), "--repo", &w.root_str()]);
    let next = said.split(':').next().unwrap().to_string();
    assert_eq!(task(&next).s("followUp"), first);
    assert_eq!(sw::actions::finish_line::details(&next).unwrap().at("followUp").s("id"), first);
    delegate(&next);
    let t = settle(&next);
    let prompt = prompts(&w, t.at("link").s("pane")).join("\n");
    assert!(prompt.contains("keep to that scope") && prompt.contains(&format!("its work on branch {branch}")), "{prompt}");
    assert!(prompt.contains("Its closing note:\n    Added the parser."), "{prompt}");
    let err = command("add", &parse(&["add", "Again", "--follow-up", &first, "--on", "vm-x"].iter().map(|s| s.to_string()).collect::<Vec<_>>())).unwrap_err().msg;
    assert!(err.contains("--follow-up names a task of this machine"), "{err}");
}

#[test]
fn research_results_are_complete_in_chat_without_queueing_or_starting_a_coordinator() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let co = Co::top(&w.socket);
    sw::coordinator::state::change(&co, |c| { c.put("stopped", true); true }).unwrap();
    let before = sw::coordinator::state::get(&co);
    let note = format!("Findings:\n{}\nFinal finding after character 4000.", "The cache is cold. ".repeat(300));
    w.script(json!({ "worker": [{ "note": note }] }));
    let (added, _) = run(&["add", "Why is the build slow", "--research", "--repo", &w.root_str()]);
    let id = added.t("id");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{t:?}");
    assert!(!t.b("report") && !sw::research::report_folder(&t).exists());
    sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes);
    let messages = sw::digest::messages(&co);
    assert!(messages.iter().any(|m| m.text.contains("Final finding after character 4000.")));
    assert_eq!(sw::coordinator::state::get(&co), before, "delivery must leave a stopped coordinator alone");
}
