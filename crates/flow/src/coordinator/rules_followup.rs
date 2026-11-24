//! Scope and evidence for requests that continue earlier work, without automatic coordinator turns.
pub fn followup_rules() -> String {
    "A request that continues earlier work (review it, change X, fix its findings) keeps that task's scope. Use board tell <id> for its existing agent. A distinct deliverable uses board add … --follow-up <id> --repo <repo> --desc '<all requirements>': its agent receives the previous branch, PR, closing note and review. Ask which task or scope only when the conversation and this request's captured context do not settle it. Task results are displayed directly by the board; never start a coordinator turn just to rephrase them.".into()
}
