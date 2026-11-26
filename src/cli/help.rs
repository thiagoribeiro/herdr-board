//! What `herdr-board help` prints: every command, in a few words each.
pub const HELP: &str = r#"herdr-board: a work board for herdr

  herdr-board                     open the board here (in herdr: bind a key to the plugin action "open")
  herdr-board add <title> [--desc TEXT] [--repo NAME|PATH] [--delegate] [--after ID[,ID…]] [--follow-up ID] [--<setting> VALUE ...] [--research|--no-research] [--chat|--no-chat] [--on MACHINE]
                         [--route ENTRY --route.reason TEXT]
                                      (--research: research only, short for --research.only true. Its agent changes
                                       nothing in the repo; the result is its closing note. --no-research: not research only,
                                       where a default says it is. --chat: chat only, short for --chat.only true: a
                                       conversation that changes nothing and ends only when you say: finish <id>.
                                       --on: add it on another machine the board knows, as that machine adds its own.
                                       --route: the catalog's entry it works on (models catalog), with why in a line)
  herdr-board list [--archived] [--all] [--pinned] [--query TEXT] [--json]
  herdr-board status [ID ...] [--all] [--repo NAME] [--json]  current owner, next action, blockers and cached result; no model call
  herdr-board summary [ID ...] [--repo NAME] [--json]        cached delivery summaries and original evidence
  herdr-board usage [ID ...] [--repo NAME] [--json]          measured new/cache/output tokens, independently of limits
  herdr-board budget [status] [--json]                    shared observed use, reservations and uncertain outcomes
  herdr-board budget settle OWNER --used TOKENS [--ended] reconcile usage; --ended confirms a lost call has ended
  herdr-board evaluate cases [--json]                     initial quality, usage and friction scenarios
  herdr-board evaluate record ID --assessment FILE [--json]  print an assessed trial; never starts an agent
  herdr-board evaluate compare BASELINE CANDIDATE [--json]   compare paired records (docs/agent-evaluation.md)
  herdr-board batch <delegate|approve|stop|retry|review|fix|archive|restore> ID ... [--request-id ID] [--json]
                                      per-task outcomes in one call; a repeated request-id reuses its receipt without repeating work
  herdr-board report <id> --state idle|working|blocked|done|error [--kind question|permission|auth] [--msg TEXT] [--title TEXT]
                         [--progress 0-100] [--eta MINUTES]   (your own task, with --state working: how far, and minutes left)
  herdr-board describe <id> <text>          write or replace a task's description
  (a title is at most 60 characters: a longer one is shortened at a word, its full text kept in the description)
  (an <id> is repo/134; every repo has a short key, made from its name, and HSW-134 names the same task, in any
   letter case. Another key: herdr-board set repo.key HSW --repo NAME; the old one still names its tasks)
  herdr-board delegate <id> [--after ID[,ID…]|--now] [--<setting> VALUE ...] [--research|--no-research] [--chat|--no-chat] [--route ENTRY --route.reason TEXT]
                                      any work setting, for this task only: --reviewer.agent claude --merge.by itself
                                      (a refused option changes nothing on the task)
                                      --after: start only once those tasks (of any repo here, or of another machine:
                                       vm-str:ELA-3) are done, by itself;
                                      --now: start a task that waits without waiting
  herdr-board after <id> [ID[,ID…] | --clear]   what a task waits for: shown, changed, or dropped (it starts then)
  herdr-board tell <id> <text>              send a message to the task's agent
  herdr-board stop <id>                     stop its agent
  herdr-board approve <id> [--anyway]       merge finished work that waits for approval (--anyway: also when its review
                                                or its pull request's checks on GitHub failed);
                                                on a task whose plan is ready (plan.approve you): yes to the plan, the build starts.
                                                tell <id> <text> there sends the plan back with those words
  herdr-board pick <id> <letter>            keep one of a task's several attempts (attempts.by); the others' tabs close
  herdr-board review <id>                   send the reviewer's findings back to the agent
  herdr-board retry <id>                    send the saved error back to the agent; on a task a limit stopped: continue
  herdr-board fix <id>                      send failure details to the agent (never approves a merge)
  herdr-board finish <id>                   finish anyway, when the agent stopped without a summary or at a limit,
                                                or when the board missed it finishing (its agent at rest, the row still "Finishing");
                                                on a chat-only task: end the conversation, as it is
  herdr-board details <id>                  failure details, review, next steps, and what it used against its limits
  herdr-board archive|restore|clear <id>
  herdr-board pin|unpin <id | machine | project> [--on MACHINE]
                                                pin a task, a machine or a project (or unpin it): it gets a yellow pin mark ★ and
                                                shows under the board's Pinned filter. --on MACHINE: a project of another machine
  herdr-board archive --done [--repo NAME]  archive every done task of that repo (or of every repo) in one go; one
                                                that works, waits, is to do or is pinned stays, and is counted
  herdr-board archive|restore --repo NAME [--on MACHINE]
                                                archive a whole project with its tasks (what works or waits is stopped
                                                first; its coordinator stops, its automations go off), or bring it back
                                                with its tasks as they were. Nothing on disk or GitHub is deleted.
                                                --on MACHINE: a project of another machine, done there
  herdr-board set <name> <value> [--repo NAME | --task ID | --automation NAME | --main-coordinator | --machine-coordinator | --project-coordinator REPO]
                                      change a setting: for you, for your entry for one repo, for one task, for an automation,
                                      or for one coordinator (its coordinator.* and autopilot.* settings)
  herdr-board unset <name> [--repo NAME | --task ID | the same coordinator flags]
                                      back to what it would be without that answer
  herdr-board show [<id>] [--repo NAME]     every setting and the place it came from; without an id, your own
                                      (a program setting, such as worker.agent, takes claude, codex, pi or another
                                       program herdr can start, as agy/gemini-3-pro; one that cannot do that role is
                                       refused with the reason)
  herdr-board models [PROGRAM]              the model names each program takes, written as a setting writes them
                                      (codex/<model>); a name it does not take is refused when it is set
  herdr-board models catalog [--init]       the catalog of models a task is routed to (~/.herdr-board/models.toml), each
                                      entry with its cost, what it is good for and how it fared; --init writes the example
  herdr-board models stats [--since 30d]    routed tasks by entry: tokens, cost, reviews failed, escalations
  herdr-board models equivalents [ENTRY]   declared same-family availability alternatives; no model is started
  herdr-board models catalog --init --profile corresponding   initialize proposed Claude/Codex pairs, without overwriting
  herdr-board automations [add|show|change|on|off|run|firings|remove] …
                                      tasks made on a schedule or when something happens on the board; each firing adds
                                      a task and puts an agent on it. `herdr-board automations help` says how
  herdr-board coordinator                   the Coordinator (the agent behind the board's box): who it is, how it stands
  herdr-board coordinator tell <text> [--delegate]
                                                say something to it (it is started when it is not running); --delegate is
                                                the board's tab: what it adds from this message gets an agent at once
  herdr-board coordinator where             where the human was when they last wrote to it from the board (for the Coordinator itself)
  herdr-board coordinator context           compact task states and cached results, without a model turn
  herdr-board coordinator usage             total and last-request token measurements, new and cached separately
  herdr-board coordinator memory [JSON]     read or save decisions, constraints and questions (string arrays, at most 12 KB)
  herdr-board coordinator history           saved session references from checkpoints
  herdr-board coordinator checkpoint        after saving memory, start a fresh chat with the same program, model and effort; history stays
  herdr-board coordinator help [TOPIC]      operating reference, one topic at a time (or all), loaded only when needed
  herdr-board coordinator restart           start its program again, in the same conversation
  herdr-board coordinator new [--agent KIND[/MODEL]] [--effort E]
                                                a fresh one; with options, that choice is for this one only
  herdr-board coordinator stop              close it; the next message starts it again where it left off (words still waiting go in then)
  herdr-board coordinator --machine …       the same for this machine's machine coordinator
  herdr-board coordinator --project REPO …  the same for a repo's project coordinator
  herdr-board autopilot                     every coordinator of this machine, with its autopilot (docs/autopilot.md)
  herdr-board autopilot show [--machine | --project REPO]
                                                one coordinator's autopilot as it stands now: goal, today's counts,
                                                its tasks under way, its settings
  herdr-board autopilot on [--machine | --project REPO] [--goal TEXT] [--autopilot.<name> VALUE …]
                                                put a coordinator on autopilot: the board wakes it to work toward its goal,
                                                within its limits; the values given are for this time only
  herdr-board autopilot off [--machine | --project REPO | --all]
                                                off again (--all: every coordinator of this machine)
  herdr-board --skill                       instructions an agent can load
  herdr-board machines                      other machines: whether each answers, its version and free disk, or why not
                                                and what to do; and whether another machine could ask this one
  herdr-board sweep [--dry]                 remove what ended tasks left behind now: their temporary files, agents'
                                                files for worktrees that are gone, old build output (docs/disk-space.md);
                                                --dry: only say what would go
  herdr-board web-ui [link|unlink]          the board inside the web UI (Ovis): how it stands; point the web UI's shared link at it, or back
  herdr-board voice [end|test]              voice calls here: where the ElevenLabs key is found (ELEVENLABS_API_KEY,
                                                or ~/.herdr-board/settings/elevenlabs.json), the call helper on this
                                                computer, how the last call sounded, a call on now; end hangs it up;
                                                test tries this computer's sound for a call (a tone, the microphone)
  herdr-board version                       this program's version, and where it is
  herdr-board install                       write the launcher in the data folder (agents and hooks run it) for this program
  herdr-board doctor                        the health check: the program and its launcher, the plugin in herdr, the
                                                agent programs and their settings, other machines, disk space, leftovers
                                                to clean, older installs; each problem with what to do (exits 1 on one)
  herdr-board update [VERSION]              this program, in place, from the newest release (or that version)
  herdr-board remove-extras                 take out what the plugin added outside its own folders (the how-to
                                                files, its blocks in herdr's config); run it before unlinking
"#;
