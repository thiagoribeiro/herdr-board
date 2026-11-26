//! What is installed on this machine, whether its turn-end hook can be used, and pre-trusting a repo.
use super::adapter;
use super::kinds::kind;
use super::read_turn::{holds, read_turn, Turn, TAIL};
use crate::clock::now;
use crate::json::*;
use crate::store::files::*;
use crate::store::marks::mark;
use std::path::{Path, PathBuf};

fn versions_file() -> PathBuf { data_dir().join("versions.json") }
/// Explicit health-check refresh: discard stale installation results and probe again in this process's environment.
pub fn refresh() -> crate::err::Res<()> { write_json(&versions_file(), &json!({})) }
/// Reuse the executable that answered the version probe, including for auth, model lists and tool-free helpers.
pub fn executable(k: &str) -> PathBuf {
    version_of(k);
    read_obj(&versions_file()).and_then(|v| v.o(k).cloned())
        .and_then(|v| v.so("executable").map(PathBuf::from)).filter(|p| crate::herdr::find::runs(p))
        .unwrap_or_else(|| PathBuf::from(super::kinds::program(k)))
}
/// How long a program is given to say its version: one that hangs holds up nobody for longer (`late`).
const VERSION_MS: u64 = 5000;
/// The installed version of a program ("" when it is not installed), asked at most every ten minutes, of its own
/// executable (cursor: cursor-agent).
pub fn version_of(k: &str) -> String {
    if k.is_empty() || !k.chars().next().unwrap().is_ascii_lowercase() || !k.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-') { return String::new(); }
    let all = read_obj(&versions_file()).unwrap_or_default();
    if all.at(k).is_object() && now() - all.at(k).i("at") < 600000 { return all.at(k).t("v"); }
    use std::process::{Command, Stdio};
    let program = super::kinds::program(k);
    let mut executable = PathBuf::from(program);
    let mut probe = |exe: &Path| {
        let r = Command::new(exe).arg("--version").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn()
            .map_err(|_| false).and_then(|c| crate::herdr::cli::wait_with_timeout(c, VERSION_MS).map_err(|_| true));
        if !matches!(r, Err(false)) { executable = if exe.is_absolute() { exe.to_path_buf() } else { crate::herdr::find::in_dirs(program, &std::env::var("PATH").unwrap_or_default()).unwrap_or_else(|| exe.to_path_buf()) }; }
        r
    };
    let mut r = probe(Path::new(program));
    // A PATH entry can exist but fail to spawn (for example an obsolete interpreter). Try the user's actual install.
    for dir in crate::herdr::find::spots(&user_home()) {
        if !matches!(r, Err(false)) { break; }
        let exe = dir.join(program); if crate::herdr::find::runs(&exe) { r = probe(&exe); }
    }
    if matches!(r, Err(false)) {
        let path = crate::login_env::login_shell().map(|o| crate::login_env::path_of(&o)).unwrap_or_default();
        for dir in path.split(':').filter(|d| !d.is_empty()) {
            let exe = Path::new(dir).join(program); if crate::herdr::find::runs(&exe) { r = probe(&exe); }
            if !matches!(r, Err(false)) { break; }
        }
    }
    let (v, found, late) = match &r {
        Ok(o) if o.status == 0 => (first_version(&o.stdout), true, false),
        Ok(_) => (String::new(), true, false),
        Err(late) => (String::new(), *late, *late),
    };
    let mut all = read_obj(&versions_file()).unwrap_or_default();
    all.put(k, if late { json!({ "v": v, "at": now(), "found": found, "late": true, "executable": executable }) } else { json!({ "v": v, "at": now(), "found": found, "executable": executable }) });
    let _ = write_json(&versions_file(), &Value::Object(all));
    v
}
fn first_version(s: &str) -> String {
    static RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r"\d+\.\d+(?:\.\d+)?").unwrap());
    RE.find(s).map(|m| m.as_str().to_string()).unwrap_or_default()
}
/// Its `--version` did not answer in time, when it was last asked.
pub fn late(k: &str) -> bool { read_obj(&versions_file()).unwrap_or_default().at(k).b("late") }
pub fn installed(k: &str) -> bool { version_of(k); read_obj(&versions_file()).unwrap_or_default().at(k).at("found") != &json!(false) }
pub fn at_least(v: &str, min: &str) -> bool {
    let a: Vec<i64> = v.split('.').map(|x| x.parse().unwrap_or(0)).collect();
    let b: Vec<i64> = min.split('.').map(|x| x.parse().unwrap_or(0)).collect();
    for i in 0..3 { let (x, y) = (a.get(i).copied().unwrap_or(0), b.get(i).copied().unwrap_or(0)); if x != y { return x > y; } }
    true
}
/// Can this kind's turn-end hook be passed at launch here? The installed version meets the adapter's minimum, and the
/// hook is not marked as not working for this program and plugin version.
pub fn hook_usable(k: &str) -> bool {
    if !adapter::has_hook(k) || std::env::var("HERDR_BOARD_NO_HOOK").as_deref() == Ok("1") { return false; }
    let v = version_of(k);
    !v.is_empty() && at_least(&v, kind(k).map(|a| a.min_version).unwrap_or("0")) && mark(&format!("hook:{k}"), &v).is_none()
}
/// Does a finished turn still need the closing note? Yes unless the hook and the reader both work for this kind.
pub fn needs_note(k: &str) -> bool { !kind(k).map(|a| a.reads_turn).unwrap_or(false) || !hook_usable(k) || mark(&format!("reader:{k}"), &version_of(k)).is_some() }
fn file_for(k: &str, file: Option<&Path>, session: &str, cwd: &Path) -> Option<PathBuf> {
    match file.filter(|f| f.exists()) { Some(f) => Some(f.to_path_buf()), None if !session.is_empty() => adapter::transcript(k, session, cwd), None => None }
}
/// The end of a task's (or its reviewer's) saved conversation, or None. `whole`: the whole file is read.
pub fn read_turn_of(k: &str, file: Option<&Path>, session: &str, cwd: &Path, whole: bool) -> Option<Turn> {
    if !kind(k).map(|a| a.reads_turn).unwrap_or(false) { return None; }
    let f = file_for(k, file, session, cwd)?;
    let mut t = read_turn(k, &f, if whole { u64::MAX } else { TAIL })?;
    t.file = Some(f);
    Some(t)
}
/// Was this text submitted to the agent since a moment: do the words stand in its saved conversation? None: nothing to read.
pub fn said_to(k: &str, file: Option<&Path>, session: &str, cwd: &Path, text: &str, since: i64) -> Option<bool> {
    if !kind(k).map(|a| a.reads_turn).unwrap_or(false) { return None; }
    let f = file_for(k, file, session, cwd)?;
    holds(&f, &super::hand_limits::as_typed(text), since).ok() // a long one went in as a line naming its file
}
/// It acts without asking first, so nobody would answer a trust dialog: `full`, `auto`, and any access of a program that
/// cannot ask.
pub fn full_access(k: &str, access: &str) -> bool { access != "ask" || kind(k).map(|a| a.access_ask.is_none()).unwrap_or(true) }
pub fn interrupt_key(k: &str) -> &'static str { kind(k).map(|a| a.interrupt).unwrap_or("esc") }
/// Best effort: a failure here only means the agent's own trust dialog shows, which the inbox handles. `root` is the
/// repo, `cwd` the folder it starts in (a worktree): pi trusts a folder and those below it, not the repo's worktrees.
pub fn pre_trust(k: &str, root: &Path, cwd: &Path, access: &str) -> bool { adapter::can_trust(k) && full_access(k, access) && adapter::trust(k, if k == "pi" { cwd } else { root }) }
