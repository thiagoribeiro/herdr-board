//! Your yes is for the commit you were shown: what the branch did since that commit, in words, when an approved merge
//! comes back to you instead of landing a commit you never saw (finish/pipeline_end.rs).
use crate::json::*;
use crate::store::repos::git_out;
use crate::sync::main_word;
use crate::text::many;
use std::path::Path;

/// None: `head` is the commit you approved (its `mergeIntent`'s, or none was recorded), and your yes stands. Else what
/// changed since, ending in what to do. `review`: the one the new commit has. One kept since the commit you approved
/// (finish/pipeline.rs, by sync::merge_only's rule) already says that main merged in is all that changed, and how.
pub fn changed_since(t: &Obj, tree: &str, head: &str, review: Option<&Value>) -> Option<String> {
    let (approved, main) = (t.at("mergeIntent").s("verifiedHead"), t.s("syncedTo"));
    if tree.is_empty() || approved.is_empty() || head.is_empty() || approved == head { return None; }
    let dir = Path::new(tree);
    let lines = |args: &[&str]| -> Vec<String> { git_out(dir, args).unwrap_or_default().lines().map(String::from).collect() };
    let files = lines(&["diff", "--name-only", approved, head]);
    let range = format!("{approved}..{head}");
    // The task's own commits since: main's, up to the one last merged in, are not counted.
    let mut args = vec!["log", "--no-merges", "--format=%h %s", range.as_str()];
    if !main.is_empty() { args.extend(["--not", main]); }
    let own = lines(&args);
    let only_main = review.map(|r| r.at("kept").s("from") == approved).unwrap_or(false);
    let what = if !own.is_empty() { format!("{}: {}", many(own.len(), "new commit"), list(&own)) } else { format!("{}{} was merged in", if only_main { "only " } else { "" }, main_word(t)) };
    Some(format!("Changed since your approval ({} → {}): {what}; {}{}. Approve again to merge it as it is now.",
        short(approved), short(head), many(files.len(), "file"), if files.is_empty() { String::new() } else { format!(" ({})", list(&files)) }))
}
fn short(c: &str) -> &str { &c[..c.len().min(7)] }
/// The first few, and how many more.
fn list(items: &[String]) -> String {
    let more = items.len().saturating_sub(3);
    format!("{}{}", items[..items.len().min(3)].join(", "), if more > 0 { format!(", {more} more") } else { String::new() })
}
