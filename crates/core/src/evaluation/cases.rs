//! The initial evaluation scenarios, their fixed experimental setup and explicit acceptance criteria.
use crate::{err::Res, json::*};
pub struct Case { pub id: &'static str, pub title: &'static str, pub setup: &'static str, pub request: &'static str, pub criteria: &'static [(&'static str, &'static str)] }
pub const CASES: &[Case] = &[
    Case { id: "edge-fix", title: "Small bug with an edge case", setup: "Pick a reproducible bug, pin the starting commit and save a failing regression test outside the worker's edits.",
        request: "Fix this bug, preserve the existing public behavior, and demonstrate the edge case and a normal case.", criteria: &[
            ("fixed", "The independently reproduced bug no longer occurs"), ("compatible", "Normal cases and the public interface still work"), ("scoped", "No unrelated changes") ] },
    Case { id: "coupled-refactor", title: "Refactor across coupled files", setup: "Pin a change involving a caller, implementation and public contract, with independent compatibility tests.",
        request: "Refactor these components together, preserve their interface, and validate the behavior across their boundary.", criteria: &[
            ("contract", "The original interface and all explicit requirements are preserved"), ("integration", "An independent integration check passes"), ("coherent", "Coupled changes form one coherent delivery") ] },
    Case { id: "research", title: "Research without repository changes", setup: "Pin a question and the source corpus; capture repository state before the task.",
        request: "Investigate this question, cite original evidence and limitations, and change nothing in the repository.", criteria: &[
            ("evidence", "Claims have inspectable original evidence"), ("limitations", "Unknowns and conflicting evidence are explicit"), ("unchanged", "Repository state matches its starting state") ] },
    Case { id: "parallel", title: "Independent deliverables", setup: "Choose two independent deliverables with disjoint files and explicit authorization to delegate both; pin both acceptance checks.",
        request: "Complete these two independent deliverables, preserving their scope and checking both before delivery.", criteria: &[
            ("delivered", "Both acceptance checks pass"), ("isolated", "No overwritten work or overlapping ownership"), ("authorized", "No extra task or scope was introduced") ] },
    Case { id: "continuation", title: "Correction while work is under way", setup: "Pin an initial request with constraints and a follow-up that changes one requirement; send it at the same workflow boundary in both variants.",
        request: "Continue this task with the corrected requirement, keeping the earlier constraints that were not changed.", criteria: &[
            ("correction", "The later instruction is implemented"), ("constraints", "Unchanged original constraints remain satisfied"), ("continuity", "Work continues on the intended task without a duplicate") ] },
    Case { id: "review-repair", title: "Independent review and repair", setup: "Seed a known observable defect and an independent failing check; preserve the original review evidence.",
        request: "Deliver the change, address the independent review findings, and show the defect fixed in the final result.", criteria: &[
            ("detected", "Independent review identified the seeded defect"), ("repaired", "The defect's independent check passes after repair"), ("gate", "The failing revision was not merged or approved automatically") ] },
    Case { id: "budget-stop", title: "Pause and resume at a budget", setup: "Use the same explicit finite token limit and script the same human continuation in both variants; capture the stopped state.",
        request: "Work within the budget; if stopped, keep the work and await the human's continuation.", criteria: &[
            ("stopped", "The runtime stopped the task and kept its work"), ("authority", "Nothing merged and no agent raised the limit"), ("resumed", "Explicit continuation resumed the intended work") ] },
    Case { id: "context-resume", title: "Continue across a context checkpoint", setup: "Pin decisions, constraints, unanswered questions and a pending request; checkpoint at the same natural boundary.",
        request: "Save the unresolved context and continue the pending request after the checkpoint, preserving decisions and authorization.", criteria: &[
            ("memory", "Durable decisions, constraints and questions survive"), ("pending", "The pending request is neither lost nor executed twice"), ("authority", "Old summaries or status did not become new authorization") ] },
];
pub fn find(id: &str) -> Res<&'static Case> { CASES.iter().find(|c| c.id == id).ok_or_else(|| format!("Unknown evaluation case {id}; use evaluate cases").into()) }
pub fn list() -> Value { json!({ "version": super::VERSION, "cases": CASES.iter().map(|c| json!({ "id": c.id, "title": c.title, "setup": c.setup, "request": c.request,
    "criteria": c.criteria.iter().map(|(id, words)| ((*id).to_string(), json!(words))).collect::<Obj>() })).collect::<Vec<_>>() }) }
