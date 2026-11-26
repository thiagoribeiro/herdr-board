//! Autopilot control-file holds, including renames, deletions, dirty files and approval attempts.
use crate::common;
use common::{*, pilot::*};
use sw::{autopilot::control_files, json::*};

#[test]
fn instruction_and_native_permission_changes_wait_for_a_human_even_after_review_passes() {
    let w = pilot_world_with(json!({ "merge.by": "itself" })); let co = proj(&w);
    ok(&["autopilot", "on", "--project", "proj", "--goal", "Docs", "--autopilot.startAtOnce", "no", "--autopilot.approve", "yes"]);
    let id = sw::autopilot::add::add(&co, &w.root, "Agent instructions", Default::default(), &[], false).unwrap().task.t("id");
    w.script(json!({ "worker": [{ "files": { "CLAUDE.md": "Use the project tools.\n" }, "commit": "instructions", "note": "Updated instructions." }], "coordinator": [{ "status": "working" }] }));
    delegate(&id); let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.s("msg"));
    assert!(t.s("msg").contains("autopilot.controlFilesWaits"));
    assert_eq!(control_files::changed(&t).unwrap(), vec!["CLAUDE.md"]);
    tick(&w); in_pane(&w, &pane_of(&co));
    assert!(refused(&["approve", &id]).contains("agent control files"));
    assert!(refused(&["set", "autopilot.controlFilesWaits", "no", "--project-coordinator", "proj"]).contains("cannot change settings"));
    in_pane(&w, "");
    ok(&["set", "autopilot.controlFilesWaits", "no", "--project-coordinator", "proj"]);
    assert!(control_files::held(&task(&id)), "the task retains the policy at creation");
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    assert_eq!(settle(&id).s("stage"), "done");
}

#[test]
fn renamed_deleted_or_dirty_control_files_cannot_disappear_from_the_hold() {
    let w = world();
    std::fs::write(w.root.join("AGENTS.md"), "Original instructions\n").unwrap();
    git(&w.root, &["add", "AGENTS.md"]); git(&w.root, &["commit", "-m", "instructions"]);
    let base = git(&w.root, &["rev-parse", "HEAD"]);
    git(&w.root, &["checkout", "-b", "control-change"]);
    git(&w.root, &["mv", "AGENTS.md", "ordinary.md"]); git(&w.root, &["commit", "-m", "rename"]);
    let t = obj(json!({ "worktree": w.root, "baseSha": base, "autopilot": {} }));
    assert!(control_files::changed(&t).unwrap().contains(&"AGENTS.md".into()));
    // An untracked native settings file with spaces is read through NUL-delimited output.
    std::fs::create_dir_all(w.root.join("nested/.claude")).unwrap();
    std::fs::write(w.root.join("nested/.claude/my hooks.json"), "{}\n").unwrap();
    assert!(control_files::changed(&t).unwrap().contains(&"nested/.claude/my hooks.json".into()));
    assert!(control_files::held(&t));
    assert!(control_files::protected("docs/AGENTS.md"));
    assert!(!control_files::protected("docs/agents.md"));
}

#[test]
fn an_unavailable_control_file_check_holds_only_autopilot_work() {
    let _w = world();
    let mut t = obj(json!({ "worktree": "/missing-worktree", "autopilot": {} }));
    assert!(control_files::held(&t));
    t.del("autopilot");
    assert!(!control_files::held(&t));
}
