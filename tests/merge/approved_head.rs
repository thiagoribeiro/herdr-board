//! Your yes is for the commit you approved: a new commit after it, or main merged in (whose review the gate keeps),
//! waits for your yes again and says what changed since; with nothing changed the approved commit merges as it is.
use crate::common;
use common::*;
use serde_json::{json, Value};
use std::path::Path;
use sw::actions::finish_line;
use sw::json::*;
use sw::store::reviews::review_history;

/// A task checked and reviewed (it passed), waiting for your yes.
fn approval(w: &World) -> String {
    let pass = json!({ "verdict": { "meets": true, "text": "Good.", "findings": [] } });
    w.script(json!({ "reviewer": [pass.clone(), pass] }));
    to_approval(w, "Work")
}
fn world() -> World { world_with(Opts { repo: true, repo_settings: Some(json!({ "check.command": "echo run >> \"$HOME/checks.log\"" })), ..Default::default() }) }
fn checks(w: &World) -> usize { read(&w.base.join("home/checks.log")).lines().count() }
fn commit(dir: &Path, file: &str, message: &str) { std::fs::write(dir.join(file), "new\n").unwrap(); git(dir, &["add", "-A"]); git(dir, &["commit", "-qm", message]); }
/// It waits for you again, with nothing on its way to merge, and nothing of it merged.
fn back_to_you(w: &World, id: &str, approved: &str) -> Obj {
    let t = task(id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure")); assert!(!t.b("mergeIntent") && !t.b("autoMerge"));
    assert_eq!((row(id).t("state"), row(id).t("kind")), ("blocked".into(), "permission".into()));
    assert!(t.s("msg").starts_with("Approve to merge?") && t.s("msg").contains(&format!("Changed since your approval ({} → {}): ", &approved[..7], &t.s("reviewedHead")[..7])), "{}", t.s("msg"));
    assert!(!w.root.join("f.txt").exists(), "nothing merged");
    t
}

#[test]
fn a_new_commit_after_your_yes_is_checked_and_reviewed_then_waits_for_your_yes_again_saying_which_commit_and_files() {
    let w = world();
    let id = approval(&w);
    let t = task(&id);
    let approved = t.t("reviewedHead");
    commit(Path::new(t.s("worktree")), "late.txt", "late work"); // the agent commits after the review you looked at
    finish_line::approve(&id, None, false).unwrap();
    settle(&id);
    let t = back_to_you(&w, &id, &approved);
    assert!(t.s("msg").contains("1 new commit: ") && t.s("msg").contains("late work; 1 file (late.txt)."), "{}", t.s("msg"));
    assert_eq!((checks(&w), review_history(&t).len()), (2, 2), "the new commit is checked and reviewed before it comes to you");
    let t = approve_again(&id, "late work");
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert!(w.root.join("late.txt").exists() && w.root.join("f.txt").exists());
    assert_eq!((checks(&w), review_history(&t).len()), (2, 2), "the commit you approved lands as it was checked and reviewed");
}

#[test]
fn main_merged_in_after_your_yes_keeps_its_review_and_is_shown_as_only_main_and_you_confirm_again() {
    let w = world();
    let id = approval(&w);
    let approved = task(&id).t("reviewedHead");
    commit(&w.root, "g.txt", "main moves"); // main moves; your yes catches the branch up with it
    finish_line::approve(&id, None, false).unwrap();
    settle(&id);
    let t = back_to_you(&w, &id, &approved);
    assert!(t.s("msg").contains("only main was merged in (1 commit of main); 1 file (g.txt)."), "{}", t.s("msg"));
    // the gate that keeps a review after a clean merge of main still does: the check runs, the review is not repeated
    assert!(t.s("msg").contains("review 1 still stands"), "{}", t.s("msg"));
    assert!(t.at("review").b("keptAt")); assert_eq!((checks(&w), review_history(&t).len()), (2, 1));
    let t = approve_again(&id, "only main was merged in");
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert!(w.root.join("f.txt").exists()); assert_eq!(checks(&w), 2);
}

#[test]
fn with_nothing_changed_since_your_yes_the_commit_you_approved_merges_at_once() {
    let w = world();
    let id = approval(&w);
    let approved = task(&id).t("reviewedHead");
    // as the board's own yes key gives it: what you were shown
    finish_line::approve(&id, Some((Value::Null, json!(approved))), false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure")); assert!(!t.s("msg").contains("Changed since"));
    assert!(w.root.join("f.txt").exists());
    assert_eq!((checks(&w), review_history(&t).len()), (1, 1), "nothing checked or reviewed again");
}
