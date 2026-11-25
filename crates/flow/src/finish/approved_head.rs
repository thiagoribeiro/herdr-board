//! Your yes is for the commit you were shown: what the branch did since that commit, in words, when an approved merge
//! comes back to you instead of landing a commit you never saw (finish/pipeline_end.rs).
use crate::json::*;
use crate::store::repos::git_out;
use crate::sync::{main_word, merge_only::merge_only};
use std::path::Path;

/// None: `head` is the commit you approved (its `mergeIntent`'s, or none was recorded), and your yes stands. Else what
/// changed since, ending in what to do. Main's own commits (up to the one last merged in) are not counted as the task's.
pub fn changed_since(t: &Obj, tree: &str, head: &str) -> Option<String> {
    let (approved, main) = (t.at("mergeIntent").s("verifiedHead"), t.s("syncedTo"));
    if tree.is_empty() || approved.is_empty() || head.is_empty() || approved == head { return None; }
    let dir = Path::new(tree);
    let lines = |args: &[&str]| -> Vec<String> { git_out(dir, args).unwrap_or_default().lines().map(String::from).collect() };
    let range = format!("{approved}..{head}");
    let files = lines(&["diff", "--name-only", approved, head]);
    let only_main = merge_only(tree, approved, main);
    let what = if only_main.b("ok") {
        let (resolved, base) = (only_main.arr("resolved").len(), main_word(t));
        format!("only {base} was merged in ({} of {base}{})", many(lines(&["rev-list", "--no-merges", &range]).len(), "commit"),
            if resolved > 0 { format!(", conflicts resolved in {}", many(resolved, "file")) } else { String::new() })
    } else {
        let mut args = vec!["log", "--no-merges", "--format=%h %s", range.as_str()];
        if !main.is_empty() { args.extend(["--not", main]); }
        let own = lines(&args);
        if own.is_empty() { only_main.t("why") } else { format!("{}: {}", many(own.len(), "new commit"), list(&own)) }
    };
    Some(format!("Changed since your approval ({} → {}): {what}; {}{}. Approve again to merge it as it is now.",
        short(approved), short(head), many(files.len(), "file"), if files.is_empty() { String::new() } else { format!(" ({})", list(&files)) }))
}
fn short(c: &str) -> &str { &c[..c.len().min(7)] }
fn many(n: usize, what: &str) -> String { format!("{n} {what}{}", if n == 1 { "" } else { "s" }) }
/// The first few, and how many more.
fn list(items: &[String]) -> String {
    let more = items.len().saturating_sub(3);
    format!("{}{}", items[..items.len().min(3)].join(", "), if more > 0 { format!(", {more} more") } else { String::new() })
}
