//! The catalog of models and a task's route: the catalog read and checked (an entry refused with why), the programs
//! it allows refused wherever they are named, a route as a place of its own in the settings lookup (`routed: …`), the
//! program you name winning over it, and `models catalog` / `models stats` (docs/models.md#catalog-and-routing).
use crate::common;
use common::*;
use serde_json::json;
use sw::agents::models::catalog::{parse, path, EXAMPLE};
use sw::cli::{flags::parse as parse_args, tasks::command};
use sw::json::*;
use sw::store::settings::lookup::{lookup, Scope};
use sw::store::settings::places::yours;
use sw::store::settings::values::check;

fn run(a: &[&str]) -> Result<(serde_json::Value, String), String> { command(a[0], &parse_args(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())).map_err(|e| e.msg) }
fn catalog(text: &str) { std::fs::write(path(), text).unwrap(); }
const TWO: &str = "[[entry]]\nname = \"leve\"\nagent = \"claude/haiku\"\neffort = \"low\"\ncost = 1\ngood_for = \"typo\"\nescalate_to = \"forte\"\n\n[[entry]]\nname = \"forte\"\nagent = \"claude/opus\"\neffort = \"high\"\ncost = 10\n";

#[test]
fn the_example_is_the_catalog_until_you_have_one_and_reads_without_a_problem() {
    let _w = world();
    let c = parse(EXAMPLE);
    assert!(c.problems.is_empty(), "{:?}", c.problems);
    assert_eq!(c.names(), vec!["leve", "padrao", "pesado"]);
    assert_eq!(c.chain("leve").iter().map(|e| e.agent.as_str()).collect::<Vec<_>>(), vec!["claude/haiku", "claude/sonnet", "claude/opus"]);
    assert!(c.allow.is_none(), "every program is allowed without allow.agents");
    let (j, said) = run(&["models", "catalog"]).unwrap();
    assert!(said.starts_with("Catalog of models: the board's example"), "{said}");
    assert!(said.contains("leve    claude/haiku · effort low · cost 1 · escalates to padrao"), "{said}");
    assert!(said.contains("Programs allowed: all (no allow.agents)"), "{said}");
    assert_eq!(j.arr("entries").len(), 3);
    run(&["models", "catalog", "--init"]).unwrap();
    assert_eq!(read(&path()), EXAMPLE);
    assert!(run(&["models", "catalog", "--init"]).unwrap_err().contains("is there already"));
    assert!(run(&["models", "catalog"]).unwrap().1.starts_with("Catalog of models: "), "read from the file now");
}

#[test]
fn an_entry_that_is_refused_is_named_with_why_and_not_used() {
    let _w = world();
    let c = parse("[[entry]]\nname = \"a\"\nagent = \"nope/x\"\n\n[[entry]]\nname = \"b\"\nagent = \"claude\"\ncolor = \"red\"\n\n[[entry]]\nname = \"c\"\nagent = \"claude/sonnet\"\nescalate_to = \"zz\"\n\n[[entry]]\nname = \"c\"\nagent = \"codex\"\n\n[[entry]]\nagent = \"claude\"\n\n[[entry]]\nname = \"d\"\nagent = \"claude\"\neffort = \"ultra\"\n");
    assert_eq!(c.names(), vec!["c"]);
    let all = c.problems.join("\n");
    for want in ["entry a: no agent program \"nope\"", "entry b: unknown key \"color\"", "entry c: escalate_to \"zz\" is no entry of the catalog", "entry c: the name is taken", "entry 5: it needs a name", "entry d: effort ultra"] {
        assert!(all.contains(want), "{want} in:\n{all}");
    }
    assert!(parse("[[entry]\nname=").problems[0].starts_with("models.toml is not valid TOML"));
    let circle = parse("[[entry]]\nname = \"a\"\nagent = \"claude\"\nescalate_to = \"b\"\n[[entry]]\nname = \"b\"\nagent = \"claude\"\nescalate_to = \"a\"\n");
    assert!(circle.problems.iter().any(|p| p.contains("goes round in a circle")), "{:?}", circle.problems);
    catalog("[[entry]]\nname = \"a\"\nagent = \"nope/x\"\n");
    let said = run(&["models", "catalog"]).unwrap().1;
    assert!(said.contains("No entries: nothing can be routed.") && said.contains("Not counted:\n  entry a: no agent program"), "{said}");
    assert!(run(&["add", "Typo", "--route", "a"]).unwrap_err().contains("No entry \"a\" in the catalog of models. Its entry is refused: entry a:"));
}

#[test]
fn without_allow_agents_any_program_passes_and_with_it_another_is_refused_everywhere() {
    let _w = world();
    let codex = "[[entry]]\nname = \"cx\"\nagent = \"codex\"\n";
    assert!(parse(codex).problems.is_empty());
    let only = format!("[allow]\nagents = [\"claude\"]\n\n{codex}");
    assert!(parse(&only).problems[0].contains("entry cx: codex is not allowed here: allow.agents allows only claude"), "{:?}", parse(&only).problems);
    catalog(&only);
    let e = check("reviewer.agent", &json!("codex"), "").unwrap_err().msg;
    assert!(e.starts_with("reviewer.agent: codex is not allowed here: allow.agents in ") && e.contains("allows only claude. Choose one of those, or add codex to that list."), "{e}");
    assert!(check("worker.fallback", &json!("claude, codex"), "").is_err());
    assert!(check("attempts.by", &json!("claude, codex"), "").is_err());
    assert_eq!(check("worker.agent", &json!("claude/opus"), "").unwrap(), json!("claude/opus"));
    assert!(run(&["add", "X", "--worker.agent", "codex"]).unwrap_err().contains("codex is not allowed here"));
    assert!(run(&["set", "reviewer.agent", "codex"]).unwrap_err().contains("codex is not allowed here"));
    // The built-in reviewer is codex: show says it is refused, and it is not started.
    let shown = run(&["show"]).unwrap().1;
    assert!(regex::Regex::new(r"reviewer\.agent +codex +refused when it starts: codex is not allowed here").unwrap().is_match(&shown), "{shown}");
    let o = sw::agents::start::StartOpts { model: String::new(), effort: String::new(), fast: None, access: "full".into(), resume: String::new(), session: String::new(), extra: vec![], folder: "/".into() };
    assert!(sw::agents::start::start_args("codex", &o).unwrap_err().msg.contains("so it is not started"));
    assert!(sw::agents::start::start_args("claude", &o).is_ok());
    // A value a hand wrote into your settings that names it does not count: the next place down is used.
    std::fs::write(sw::store::files::config_dir().join("config.json"), json!({ "worker.agent": "codex", "watcher.agent": "none", "coordinator.agent": "none" }).to_string()).unwrap();
    assert_eq!(lookup("worker.agent", &Scope::you(), &yours()).unwrap().value, json!("claude"));
    assert!(yours().problems.iter().any(|p| p.why.contains("codex is not allowed here")));
}

#[test]
fn a_route_is_a_place_of_its_own_and_the_program_you_name_wins() {
    let w = world();
    catalog(TWO);
    let repo = w.root_str();
    let (j, said) = run(&["add", "Small refactor", "--repo", &repo, "--route", "leve", "--route.reason", "refator pequeno, 2 arquivos"]).unwrap();
    assert!(said.contains(": added, routed to leve"), "{said}");
    let id = &j.t("id");
    let t = task(id);
    assert_eq!(t.at("route").s("entry"), "leve");
    let at = |n: &str| lookup(n, &Scope::task(&t), &yours()).unwrap();
    assert_eq!((at("worker.agent").value, at("worker.agent").from), (json!("claude/haiku"), "routed: leve — \"refator pequeno, 2 arquivos\"".to_string()));
    assert_eq!(at("worker.effort").value, json!("low"));
    assert_eq!(at("worker.fallback").value, json!(["claude/opus"]));
    let shown = run(&["show", id]).unwrap().1;
    assert!(shown.contains("worker.agent") && shown.contains("claude/haiku") && shown.contains("routed: leve — \"refator pequeno, 2 arquivos\""), "{shown}");
    // What you name for the task wins: the route is not read at all.
    run(&["set", "worker.agent", "claude/opus", "--task", id]).unwrap();
    let t = task(id);
    assert_eq!(lookup("worker.agent", &Scope::task(&t), &yours()).unwrap().from, "this task");
    assert_eq!(lookup("worker.effort", &Scope::task(&t), &yours()).unwrap().value, json!("default"), "the route's effort goes with its program");
    let (j, said) = run(&["add", "With opus", "--repo", &repo, "--worker.agent", "claude/opus", "--route", "leve"]).unwrap();
    assert!(said.contains("--route leve not used: worker.agent was named for this task"), "{said}");
    assert!(task(&j.t("id")).at("route").is_null());
    let before = sw::store::tasks::all().len();
    assert!(run(&["add", "Bad", "--repo", &repo, "--route", "levve"]).unwrap_err().contains("No entry \"levve\" in the catalog of models. Did you mean leve?"));
    assert_eq!(sw::store::tasks::all().len(), before, "a refused --route adds nothing");
}

#[test]
fn route_default_is_given_at_delegation_under_coordinator_mode_or_when_the_task_says_it() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    catalog(TWO);
    w.script(json!({ "worker": [{ "status": "working" }, { "status": "working" }, { "status": "working" }] }));
    run(&["set", "route.default", "forte"]).unwrap();
    let a = w.add("Not routed");
    delegate(&a);
    assert!(task(&a).at("route").is_null(), "route.mode none: route.default is not read");
    run(&["set", "route.mode", "coordinator"]).unwrap();
    let b = w.add("Routed by default");
    delegate(&b);
    let t = task(&b);
    assert_eq!((t.at("route").s("entry"), t.at("route").s("by")), ("forte", "default"));
    assert_eq!(lookup("worker.agent", &Scope::task(&t), &yours()).unwrap().from, "routed: forte (route.default)");
    run(&["set", "route.mode", "none"]).unwrap();
    let c = w.add("Its own");
    run(&["set", "route.default", "leve", "--task", &c]).unwrap();
    delegate(&c);
    assert_eq!(task(&c).at("route").s("entry"), "leve", "said on the task itself, it counts whatever route.mode says");
    run(&["set", "route.default", "nada"]).unwrap();
    assert!(run(&["show"]).unwrap().1.contains("not used: no such entry in the catalog of models"));
}

#[test]
fn models_stats_counts_the_routed_tasks_by_the_entry_first_chosen() {
    let w = world();
    catalog(TWO);
    let now = sw::clock::now();
    let route = |first: &str, entry: &str, ups: usize, at: i64| json!({ "entry": entry, "first": first, "reason": "", "by": "coordinator", "at": at, "chain": [{ "name": "leve", "agent": "claude/haiku", "effort": "low", "cost": 1 }, { "name": "forte", "agent": "claude/opus", "effort": "high", "cost": 10 }], "escalated": vec![json!({ "from": "leve", "to": "forte" }); ups] });
    let fail = json!([{ "round": 1, "pass": false }]);
    for (first, entry, ups, spent, reviews, at) in [("leve", "forte", 1, 200_000, fail.clone(), now), ("leve", "leve", 0, 100_000, json!([]), now), ("forte", "forte", 0, 0, fail.clone(), now), ("leve", "leve", 0, 0, json!([]), now - 40 * 86_400_000)] {
        let id = w.add("t");
        sw::store::tasks::set(&id, json!({ "route": route(first, entry, ups, at), "spent": { "total": spent }, "reviews": reviews })).unwrap();
    }
    let (j, said) = run(&["models", "stats"]).unwrap();
    let leve = j.arr("entries").iter().find(|e| e.s("entry") == "leve").cloned().unwrap();
    assert_eq!((leve.i("tasks"), leve.f("tokensEach"), leve.f("escalated"), leve.f("reviewFailed"), leve.f("costInAll")), (2, 150_000.0, 0.5, 0.5, 2.0), "{said}");
    assert!(regex::Regex::new(r"\nleve +2 +150k +1 +2 +50% +50%\n").unwrap().is_match(&said), "{said}");
    assert!(said.contains("leve: escalated 50% in proj"), "{said}");
    assert!(said.contains("tokens/task is over the tasks whose tokens could be counted"), "{said}");
    assert_eq!(run(&["models", "stats", "--since", "all"]).unwrap().0.arr("entries").iter().find(|e| e.s("entry") == "leve").unwrap().i("tasks"), 3);
    assert!(run(&["models", "stats", "--since", "soon"]).unwrap_err().contains("--since is a number of days"));
    assert!(run(&["models", "catalog"]).unwrap().1.contains("used: 2 tasks in 30 days · review failed 50% · escalated 50% · ~150k tokens a task · escalated 50% in proj"));
}

#[test]
fn the_coordinators_rule_on_a_model_is_the_catalog_only_under_coordinator_mode() {
    let _w = world();
    let rules = || sw::coordinator::rules::rules("board");
    let r = rules();
    assert!(r.contains("When the human names no model, give none: the program alone (worker.agent codex), never a model of your own choosing"), "{r}");
    assert!(!r.contains("--route"));
    run(&["set", "route.mode", "coordinator"]).unwrap();
    let r = rules();
    assert!(!r.contains("give none: the program alone"));
    for want in ["run \"board models catalog\"", "--route <entry> --route.reason \"<why, one line", "Never a model of your own choosing, never an entry that is not in the catalog", "always wins over routing", "The plan and the review are never routed"] {
        assert!(r.contains(want), "{want}");
    }
}
