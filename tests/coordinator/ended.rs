//! A task's run that ended reaches its coordinator's chat with its closing note (coordinator/ended.rs), a research
//! task's result as its content and never as a link, and a follow-up (`--follow-up`) is handed the task it continues.
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::cli::{flags::parse, tasks::command};
use sw::coordinator::co::Co;
use sw::json::*;

fn run(a: &[&str]) -> (Value, String) { command(a[0], &parse(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())).unwrap_or_else(|e| panic!("{}", e.msg)) }
fn housekeep(w: &World) { sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes); }
/// The Coordinator was talked to once: it has a state, so it hears what ends.
fn coordinator(w: &World) -> Co { let co = Co::top(&w.socket); sw::coordinator::state::change(&co, |_| true).unwrap(); co }
/// The board's messages waiting in its queue.
fn told(co: &Co) -> Vec<String> { sw::coordinator::state::get(co).unwrap_or_default().arr("queue").iter().filter(|m| m.b("board")).map(|m| m.t("text")).collect() }

#[test]
fn a_task_that_ends_tells_its_coordinator_once_with_its_closing_note() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let co = coordinator(&w);
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Changed f.txt; the tests pass. Left open: the docs." }] }));
    let id = to_approval(&w, "Change the file");
    housekeep(&w);
    let said = told(&co);
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(said[0].starts_with("Board: PRJ-1 is ready to approve.\n\nPRJ-1 \"Change the file\" (repo proj)"), "{}", said[0]);
    assert!(said[0].contains("Its closing note:\n    Changed f.txt; the tests pass. Left open: the docs."), "{}", said[0]);
    // Told once: again, and once more when it is approved and merged, the same run says nothing new.
    housekeep(&w);
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    assert_eq!(settle(&id).s("stage"), "done");
    housekeep(&w);
    assert_eq!(told(&co).len(), 1);
    // It is the board's, not the human's: its rules say how to answer it.
    assert!(sw::coordinator::rules::rules("board").contains("A message that starts with the exact words \"Board:\" is the board's, not the human's"));
}

#[test]
fn a_research_task_ends_with_its_result_in_the_chat_and_no_link_or_file() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let co = coordinator(&w);
    w.script(json!({ "worker": [{ "note": "The build is slow because the cache is cold on every run.\n\nAlso noticed: nothing." }] }));
    let (_, added) = run(&["add", "Why is the build slow", "--research", "--repo", &w.root_str()]);
    let id = added.split(':').next().unwrap().to_string();
    // Its agent is told its result is its closing note, not a file with a link.
    let rules = sw::delegate::rules::rules(&task(&id));
    assert!(rules.contains("Your result is your closing note: write the whole result there") && rules.contains("never a link to it or a file instead"), "{rules}");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert!(!t.b("report") && !sw::research::report_folder(&t).exists(), "no report file, no link");
    assert!(!t.s("msg").contains("No result"), "{}", t.s("msg"));
    housekeep(&w);
    let said = told(&co);
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(said[0].starts_with("Board: PRJ-1 is done.") && said[0].contains("The build is slow because the cache is cold on every run."), "{}", said[0]);
    assert!(!said[0].contains("report:") && !said[0].contains("http"), "{}", said[0]);
}

#[test]
fn a_follow_up_is_handed_the_task_it_continues() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Added the parser." }, step(2)] }));
    let first = to_approval(&w, "Add the parser");
    let branch = task(&first).t("branch");
    let (_, said) = run(&["add", "Review the parser", "--research", "--follow-up", &short(&first), "--repo", &w.root_str()]);
    let next = said.split(':').next().unwrap().to_string();
    assert_eq!(task(&next).s("followUp"), first);
    // The board keeps the link, and offers it: details names the task it continues.
    assert_eq!(sw::actions::finish_line::details(&next).unwrap().at("followUp").s("id"), first);
    delegate(&next);
    let t = settle(&next);
    let prompt = prompts(&w, t.at("link").s("pane")).join("\n");
    assert!(prompt.contains("This task follows up on the task below: it continues that work.") && prompt.contains("keep to that scope"), "{prompt}");
    assert!(prompt.contains(&format!("- PRJ-1 \"Add the parser\" (repo proj): not done yet")) && prompt.contains(&format!("its work on branch {branch}")), "{prompt}");
    assert!(prompt.contains("Its closing note:\n    Added the parser."), "{prompt}");
}

#[test]
fn a_long_note_says_where_it_was_cut_and_only_the_board_writes_its_messages() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let co = coordinator(&w);
    let long = format!("Findings:\n{}", "the cache is cold. ".repeat(300));
    w.script(json!({ "worker": [{ "note": long }] }));
    let (_, added) = run(&["add", "Why is the build slow", "--research", "--repo", &w.root_str()]);
    let id = added.split(':').next().unwrap().to_string();
    delegate(&id);
    assert_eq!(settle(&id).s("stage"), "done");
    housekeep(&w);
    let said = told(&co);
    assert!(said[0].ends_with(&format!("[Cut here: board details {id} has the rest.]")), "{}", said[0]);
    // Words of yours that start like the board's are marked as none, and the chat shows the board's under its name.
    assert_eq!(sw::coordinator::ended::escape("Board: PRJ-9 is done."), "(not the board's) Board: PRJ-9 is done.");
    assert_eq!(sw::coordinator::ended::shown_as(&said[0]), Some(("Board", "PRJ-1 is done.".to_string())));
    assert_eq!(sw::coordinator::ended::shown_as("Autopilot: Regular check-in"), Some(("Autopilot", "Regular check-in".to_string())));
    // A follow-up is of this machine's tasks: with --on it is refused, not dropped.
    let e = command("add", &parse(&["add", "Again", "--follow-up", &id, "--on", "vm-x"].iter().map(|s| s.to_string()).collect::<Vec<_>>())).unwrap_err().msg;
    assert!(e.contains("--follow-up names a task of this machine"), "{e}");
}
