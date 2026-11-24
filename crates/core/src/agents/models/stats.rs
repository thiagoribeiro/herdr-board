//! `herdr-board models stats [--since 30d]`: how the routed tasks fared, by the entry of the catalog they were first
//! given: how many, their tokens (as counted for `limit.tokens`), the entry's relative cost, how many had a review fail
//! and how many escalated, and where they escalated most. The Coordinator gets a short line of it per entry with the
//! catalog, to choose better next time (docs/models.md#catalog-and-routing).
use super::catalog::Catalog;
use crate::clock::now;
use crate::err::Res;
use crate::json::*;
use crate::store::reviews::review_history;
use crate::store::settings::values::tokens_say;
use crate::store::tasks::{list, ListOpts, Task};
use std::collections::BTreeMap;

const DAY: i64 = 86_400_000;
/// One entry's tasks.
#[derive(Clone, Debug, Default)]
pub struct Row { pub entry: String, pub tasks: i64, pub tokens: f64, pub counted: i64, pub cost: f64, pub failed: i64, pub escalated: i64, pub repos: BTreeMap<String, (i64, i64)> }
impl Row {
    pub fn tokens_each(&self) -> Option<f64> { (self.counted > 0).then(|| self.tokens / self.counted as f64) }
    fn rate(&self, n: i64) -> f64 { if self.tasks == 0 { 0.0 } else { n as f64 / self.tasks as f64 } }
    pub fn json(&self) -> Value {
        json!({ "entry": self.entry, "tasks": self.tasks, "tokensEach": self.tokens_each().map(|x| x.round()), "cost": self.cost, "costInAll": self.cost * self.tasks as f64,
            "reviewFailed": self.rate(self.failed), "escalated": self.rate(self.escalated), "byRepo": self.repos.iter().map(|(r, (n, up))| (r.clone(), json!({ "tasks": n, "escalated": up }))).collect::<Obj>() })
    }
}
/// `30d`, `12h`, `all`: since when, in ms; 0 for all.
pub fn since_of(s: &str) -> Res<i64> {
    let w = s.trim().to_lowercase();
    if w.is_empty() { return Ok(now() - 30 * DAY); }
    if w == "all" { return Ok(0); }
    let (n, u) = w.split_at(w.len() - 1);
    match (n.parse::<i64>(), u) { (Ok(n), "d") if n > 0 => Ok(now() - n * DAY), (Ok(n), "h") if n > 0 => Ok(now() - n * 3_600_000), _ => crate::refuse!("--since is a number of days or hours (30d, 12h), or all; not \"{s}\".") }
}
fn pct(x: f64) -> String { format!("{}%", (x * 100.0).round() as i64) }
/// The routed tasks given since then, one row per entry first chosen; the catalog's entries first, in its order.
pub fn rows(tasks: &[Task], since: i64, c: &Catalog) -> Vec<Row> {
    let mut by: BTreeMap<String, Row> = BTreeMap::new();
    for t in tasks.iter().filter(|t| t.at("route").is_object() && t.at("route").f("at") as i64 >= since) {
        let r = t.at("route");
        let e = r.t("first");
        let row = by.entry(e.clone()).or_insert_with(|| Row { entry: e.clone(), ..Default::default() });
        row.tasks += 1;
        row.cost = r.arr("chain").first().map(|x| x.f("cost")).unwrap_or(0.0);
        let spent = t.at("spent").f("total");
        if spent > 0.0 { row.tokens += spent; row.counted += 1; }
        if review_history(t).iter().any(|v| v.at("pass") == &json!(false)) { row.failed += 1; }
        let up = !r.arr("escalated").is_empty();
        if up { row.escalated += 1; }
        let repo = if t.s("repo").is_empty() { t.s("id").split('/').next().unwrap_or("").to_string() } else { t.t("repo") };
        let x = row.repos.entry(repo).or_insert((0, 0));
        x.0 += 1;
        if up { x.1 += 1; }
    }
    let mut out: Vec<Row> = c.entries.iter().filter_map(|e| by.remove(&e.name)).collect();
    out.extend(by.into_values());
    out
}
/// Where an entry escalated most: a repo with two tasks or more of it, two of five or more of them escalated.
fn hot(r: &Row) -> Vec<String> {
    r.repos.iter().filter(|(_, (n, up))| *n >= 2 && *up as f64 / *n as f64 >= 0.4).map(|(repo, (n, up))| format!("escalated {} in {repo}", pct(*up as f64 / *n as f64))).collect()
}
/// One line per entry for the catalog, over the last 30 days: what the Coordinator reads beside it.
pub fn summary(c: &Catalog) -> BTreeMap<String, String> {
    let all = list(ListOpts { include_archived: true, ..Default::default() });
    rows(&all, now() - 30 * DAY, c).into_iter().map(|r| {
        let mut parts = vec![format!("{} task{} in 30 days", r.tasks, if r.tasks == 1 { "" } else { "s" }), format!("review failed {}", pct(r.rate(r.failed))), format!("escalated {}", pct(r.rate(r.escalated)))];
        if let Some(t) = r.tokens_each() { parts.push(format!("~{} tokens a task", tokens_say(t))); }
        parts.extend(hot(&r));
        (r.entry.clone(), parts.join(" · "))
    }).collect()
}
pub fn stats_command(since: &str, c: &Catalog) -> Res<(Value, String)> {
    let from = since_of(since)?;
    let all = list(ListOpts { include_archived: true, ..Default::default() });
    let rows = rows(&all, from, c);
    let span = if from == 0 { "ever".to_string() } else if since.trim().is_empty() { "in the last 30 days".into() } else { format!("in the last {}", since.trim()) };
    if rows.is_empty() { return Ok((json!({ "since": from, "entries": [] }), format!("No routed tasks {span}. A task is routed by --route on add or delegate, or by route.default (herdr-board models catalog)."))); }
    let wide = rows.iter().map(|r| r.entry.len()).max().unwrap_or(5).max(5);
    let mut lines = vec![format!("Routed tasks {span}, by the entry first chosen:"), String::new(), format!("{:<wide$}  {:>5}  {:>11}  {:>4}  {:>11}  {:>13}  {:>9}", "entry", "tasks", "tokens/task", "cost", "cost in all", "review failed", "escalated")];
    for r in &rows {
        let num = |x: f64| if x.fract() == 0.0 { format!("{}", x as i64) } else { format!("{x:.1}") };
        lines.push(format!("{:<wide$}  {:>5}  {:>11}  {:>4}  {:>11}  {:>13}  {:>9}", r.entry, r.tasks, r.tokens_each().map(tokens_say).unwrap_or_else(|| "-".into()), num(r.cost), num(r.cost * r.tasks as f64), pct(r.rate(r.failed)), pct(r.rate(r.escalated))));
    }
    let hot: Vec<String> = rows.iter().flat_map(|r| hot(r).into_iter().map(move |h| format!("{}: {h}", r.entry))).collect();
    if !hot.is_empty() { lines.push(String::new()); lines.extend(hot); }
    if rows.iter().any(|r| r.counted < r.tasks) { lines.push(String::new()); lines.push("tokens/task is over the tasks whose tokens could be counted (a program that cannot count them has none).".into()); }
    Ok((json!({ "since": from, "entries": rows.iter().map(Row::json).collect::<Vec<_>>() }), lines.join("\n")))
}
