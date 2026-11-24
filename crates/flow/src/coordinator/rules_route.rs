//! The Coordinator's rule on the model of a task (docs/models.md#catalog-and-routing). With `route.mode none` (the
//! default): no model of its own choosing, ever. With `route.mode coordinator`: it routes each task it adds or
//! delegates with no model named by the human to one entry of the catalog of models, by what that entry is good and
//! bad for, says why in a line, and never leaves the catalog. Plans and reviews are never routed.
use super::co::{Co, Level};
use crate::store::settings::lookup::{lookup, Scope};
use crate::store::settings::places::yours;
use crate::json::text;

/// Routing is on for this coordinator's tasks: `route.mode` for its repo (a project's coordinator) or for you.
pub fn routing(co: &Co) -> bool {
    let scope = match &co.level { Level::Project(r) => Scope::repo(r), _ => Scope::you() };
    lookup("route.mode", &scope, &yours()).map(|f| text(&f.value) == "coordinator").unwrap_or(false)
}
/// The sentence on a model the human did not name.
pub fn model_rule(co: &Co) -> String {
    if !routing(co) { return "When the human names no model, give none: the program alone (worker.agent codex), never a model of your own choosing, and the program uses its own default.".into(); }
    "When the human names no model, route the task instead (route.mode is coordinator): run \"board models catalog\" once per conversation, or again when it may have changed. It lists the catalog's entries, each with its program and effort, its relative cost, what it is good for and what to avoid it for, roughly how big a task it is for, and how it fared in the last 30 days (how often its reviews failed and it escalated, and where). For each task you add or delegate, choose the one entry whose good_for fits the work and whose avoid_for does not, the cheapest that will do it well, and pass it with the reason in one line: board add \"<title>\" … --route <entry> --route.reason \"<why, one line: e.g. small refactor, 2 files>\" (or board delegate <id> --route <entry> --route.reason \"…\"). The board makes that entry the task's worker.agent and worker.effort, and the entries it escalates to its worker.fallback: a cheap entry whose work fails review is taken over by the next one by itself. Never a model of your own choosing, never an entry that is not in the catalog, and no --worker.agent or --worker.effort beside --route. What the human names (\"com opus\", \"with codex\", \"on high effort\") always wins over routing: pass that as --worker.agent / --worker.effort, with no --route. The plan and the review are never routed: they stay on the strong models that are set, and that strong check is what makes a cheap worker safe.".into()
}
