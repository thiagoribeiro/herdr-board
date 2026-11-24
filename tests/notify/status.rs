//! `herdr-board status`: one line per active task with where it stands, for how long, whose turn it is and the next
//! step; `--json` for scripts; done tasks only with `--all` (docs/status.md).
use crate::common;
use common::*;
use sw::json::*;

fn status(w: &World, all: bool) -> (Value, String) {
    let argv: Vec<String> = if all { vec!["status".into(), "--all".into()] } else { vec!["status".into()] };
    let a = sw::cli::flags::parse(&argv);
    sw::cli::flags::refuse_unknown("status", &a).unwrap();
    let was = std::env::current_dir().unwrap();
    std::env::set_current_dir(&w.root).unwrap();
    let out = sw::cli::tasks::command("status", &a).unwrap();
    std::env::set_current_dir(was).unwrap();
    out
}

#[test]
fn status_says_each_active_task_whose_turn_it_is_and_its_next_step() {
    let w = world();
    assert_eq!(status(&w, false).1, "No active tasks.");
    let ready = to_approval(&w, "Ready one");
    let done = w.add("Old one");
    sw::store::tasks::patch(&done, |t| { t.put("stage", "done"); }).unwrap();
    sw::clock::advance(3 * 60000);
    let (v, text) = status(&w, false);
    assert_eq!(v.as_array().map(|a| a.len()), Some(1), "{text}");
    let line = text.lines().next().unwrap_or("");
    assert!(regex::Regex::new(r"^PRJ-1 +Ready one +ready to approve +3m +you +approve the merge").unwrap().is_match(line), "{line}");
    let r = &v[0];
    assert_eq!((r.s("id"), r.s("whose"), r.s("for")), (ready.as_str(), "you", "3m"));
    assert!(r.s("next").starts_with("approve the merge"), "{r}");
    // --all: the done one too
    let (v, text) = status(&w, true);
    assert_eq!(v.as_array().map(|a| a.len()), Some(2), "{text}");
    assert!(text.contains("PRJ-2"), "{text}");
    // a task nobody started is yours to delegate, as herdr's sidebar has it
    let todo = w.add("New one");
    let r = status(&w, false).0.as_array().unwrap().iter().find(|r| r.s("id") == todo).cloned().unwrap();
    assert_eq!((r.s("whose"), r.s("next")), ("you", "delegate it"));
}

#[test]
fn a_task_stalled_in_finishing_is_yours_to_look_at_not_its_agents() {
    let w = world();
    let id = to_approval(&w, "Stuck");
    sw::store::tasks::patch(&id, |t| { t.put("stage", "working"); }).unwrap();
    sw::clock::advance(14 * 3600000);
    let (v, text) = status(&w, false);
    let r = &v[0];
    assert_eq!((r.s("whose"), r.s("for"), r.at("stall").s("event")), ("you", "14h", "stalled"), "{text}");
    assert!(r.s("next").starts_with("No change for 14h"), "{text}");
}
