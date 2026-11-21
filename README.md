# Devagatchi 🐾

An offline terminal companion for developers. Feed it with Git activity, give it a name and a color, and choose cozy or traditional care. No AI at runtime, accounts, telemetry, network features, or third-party Rust dependencies.

## Install or update

Requires macOS or Linux, Git, and **Rust 1.89+** with Cargo. Check `git --version` and `cargo --version`. If needed, install Rust using the [official instructions](https://www.rust-lang.org/tools/install). Downloading tools and cloning the project require internet; the pet runs offline.

```sh
git clone https://github.com/paul-pilipczuk/Devagatchi.git
cd Devagatchi
cargo install --path . --offline --force
```

For an existing checkout, update its source before running the same install command. Editing or pulling Rust source does not update the installed executable until you rebuild/install it. Your pets are stored separately and survive updates. Do not rerun `init` for an existing pet.

If `devagatchi` is not found, run `source "$HOME/.cargo/env"` or add your Cargo `bin` directory to `PATH`. Run `devagatchi --version` to verify the installed version.

## Set up once

In the repository you work on:

```sh
devagatchi init Mochi --global
devagatchi hooks install
devagatchi color magenta --global
```

Now use `git commit` normally. The post-commit hook feeds your pet and displays its food bar. If an agent bypasses hooks, run `devagatchi sync` to catch up. No background process is required.

If you already have a global pet, skip `init` and run `hooks install` in each repository you want connected. Existing history is the baseline when a new repository is connected; it is not rewarded. An already-connected repository retains its pending work.

You can instead adopt a repository pet, or keep both:

```sh
devagatchi init Scout --repo
devagatchi status
devagatchi mode traditional --repo
```

`init NAME [PATH] [cozy|traditional] [--global|--repo]` also supports a repository path and initial mode. Defaults are the current directory, cozy mode, and global scope. Quote names and paths with spaces.

## Global and repository pets

| Scope | Storage | Activity |
| --- | --- | --- |
| Global | `$XDG_DATA_HOME/devagatchi/pet.txt`, or `~/.local/share/devagatchi/pet.txt` | Explicitly connected repositories |
| Repository | `.devagatchi.local` at the checkout root | That checkout |

`devagatchi connect` connects the current repository to the global pet without installing hooks. Hook installation connects it too. Nothing scans your computer for repositories.

If both pets exist, the same commit can feed **both**, without splitting the reward. Each pet has its own daily allowance, fullness, and care mode. Your global pet may earn a smaller incremental reward if it has already received activity from other repositories that day.

Repository state, its backup, temporary file, and lock are added to Git's local exclude file during initialization. They are not intended to be committed. A linked worktree can have its own repository pet; connected worktrees share the global pet's commit identity so the same commit is not rewarded twice. Separate clones are separate repository identities.

## Food rules

Food ranges from **0 to 100**. Additions and deletions have equal value. Each pet's daily food allowance is calculated from cumulative eligible changed lines:

```text
L = added + deleted lines, including manual feeds, for one UTC day
Daily food earned = min(100, floor(4 × sqrt(L)))
New reward = updated daily allowance − already earned daily allowance
```

| Daily eligible lines | Total food earned |
| ---: | ---: |
| 1 | 4 |
| 10 | 12 |
| 25 | 20 |
| 100 | 40 |
| 400 | 80 |
| 625+ | 100 |

Splitting the same number of lines across multiple commits on the same day does not increase the reward. Food above 100 is discarded, not banked. Food earned still consumes the daily allowance when the pet is full. This is a game, not a code-quality or productivity metric.

- Text code, tests, and documentation count.
- Binary files, whitespace-only edits, pure detected renames, and empty commits earn no food.
- Merge commits do not earn another meal for their changes; previously unseen ordinary ancestor commits can count. Merge-resolution-only changes are not rewarded in this version.
- Known generated/dependency directories (`node_modules`, `vendor`, `target`, `dist`, `build`), common lockfiles, source maps, and minified JS/CSS are excluded. These heuristics cannot identify every generated file.
- Use `devagatchi exclude generated --global` to exclude another repository-relative file or directory prefix from future meals. Exclusions are per pet and apply to its connected repositories; they are literal prefixes, not globs. They do not undo past rewards.
- Human and agent commits are treated equally. Activity is repository-wide, not filtered by author. Imported work after connection can count too.
- Manual `feed 25` adds 25 eligible lines to today's allowance; it does **not** directly add 25 food or refill the bar.

The ledger records commit IDs, counts, timestamps, and small settings. Paths are used transiently to filter statistics; source contents and patches are not stored. Commit time determines the reward day and decay, clamped between repository enrollment and the current time. Consequently catching up old activity does not give a fresh full meal today. Commit timestamps are not a tamper-proof measure of work.

## Food bar and care

A full bar lasts **72 hours in cozy** or **24 hours in traditional** with no further meals. Decay is continuous and calculated when a command runs, including time while the app is closed.

| Food | Cozy | Traditional |
| --- | --- | --- |
| 80–100 | Happy | Happy |
| 40–less than 80 | Content | Content |
| Above 0–less than 40 | Sleepy | Hungry |
| 0 | Sleepy / resting | Very hungry |

Cozy pets never become hungry. Neither mode causes death or lost progress. Feeding adds food rather than resetting a timer. Switching mode applies the old decay rate up to the change, then the new rate going forward. It does not refill food.

The current mood controls the animal's expression. The food bar appears directly under the portrait. Status shows the estimated time to empty and lifetime statistics. Colors appear in interactive terminals; redirected output, `TERM=dumb`, and a nonempty `NO_COLOR` disable them. Hooks show compact output to noninteractive clients such as agents.

## Commands

| Command | Behavior |
| --- | --- |
| `devagatchi` / `devagatchi status` | Show applicable global and repository pets |
| `devagatchi init Mochi --global` | Adopt a global pet and connect this repo |
| `devagatchi init Scout --repo` | Adopt a pet in this checkout |
| `devagatchi connect` | Connect this checkout to the global pet |
| `devagatchi sync` | Process this repo for both applicable pets; outside a repo, process global connections |
| `devagatchi sync --global` | Process all repositories connected to the global pet |
| `devagatchi sync --repo` | Process only the current repository pet |
| `devagatchi sync --catch-up` | Process up to 200 pending commits per repo; repeat if more remain |
| `devagatchi feed 25 --both` | Feed 25 manual lines to each pet's daily allowance |
| `devagatchi mode cozy --global` | Set cozy or traditional decay |
| `devagatchi pet dog --repo` | Choose cat, dog, bird, or lizard |
| `devagatchi color green --global` | Choose cyan, green, yellow, blue, magenta, red, white, or off |
| `devagatchi exclude generated --global` | Add a prefix exclusion for future work |
| `devagatchi reconcile` | Baseline this checkout's current history without rewarding pending work |
| `devagatchi hooks install` | Install commit, merge, and rewrite hooks |
| `devagatchi hooks install --push` | Also display status **before** pushes; no extra push reward |
| `devagatchi hooks status` | Inspect the effective hooks directory |
| `devagatchi hooks uninstall` | Remove only unchanged hooks owned by Devagatchi |
| `devagatchi --help` / `devagatchi help sync` | Full help / command-specific help |

Use `--global`, `--repo`, or `--both` for status and settings. If both pets exist, manual feeding and settings changes require an explicit scope. `--both` requires both pets. Automatic Git feeding updates each pet independently: if one is busy, a later sync can retry it without feeding the other twice.

## Hooks, history, and resource use

Hooks are opt-in per repository. Installation refuses to overwrite existing hooks or replace a configured `core.hooksPath`; use your existing hook manager's integration instead:

```sh
# In post-commit and post-merge:
devagatchi hook commit || :
# In post-rewrite, preserving Git's stdin and arguments:
devagatchi hook rewrite "$@" || :
# Optional pre-push status only:
devagatchi hook push || :
```

Keep your existing hooks' behavior and exit status when integrating these calls. Git has no standard local post-push hook, so the optional push display runs before the push and does not claim success.

Recorded amendment/rebase mappings mark rewritten commits as already handled, without an extra reward. Additional edits made while amending an already rewarded commit are conservatively not rewarded. Unrecognized rewritten/divergent history stops automatic rewards instead of guessing; `reconcile` explicitly skips pending history while preserving the pet. Repeated syncs never reward the same recorded commit again. Arbitrary history rewrites and cherry-picks with new IDs are not a general plagiarism or duplicate-patch detector.

- No daemon, background watcher, polling, network calls, or runtime packages.
- One short-lived hook supervisor/worker. Hook work has a 1-second budget; failures never reject Git operations. On Unix a timed-out worker and its Git subprocess group are terminated. A later `sync --catch-up` retries unfinished work.
- Normal sync refuses ranges above 200 commits. Explicit catch-up processes 200 unprocessed commits per repository per call. Individual Git subprocesses have a 5-second limit and output is capped at 16 MiB. Large files/history can be deferred or rejected rather than consuming unbounded resources.
- Statistics are reused between pets when exclusions match. Git only computes diffs for unprocessed commits; status does not diff files.
- OS file locks and atomic replacement protect concurrent updates. A crash releases the lock automatically; the small `.lock` file can remain and is not evidence of a stuck process.
- The on-disk ledger grows with tracked activity, and local status/reward calculation reads that ledger. There is no database server. This first version does not yet compact old ledger entries.

## Upgrading from 0.3

Existing global pets are recognized automatically. Name, appearance, color, tracked repository, and lifetime totals are retained. Initial food is derived from the old last-fed timestamp using the chosen mode's full-bar duration; the previous timed moods are replaced by food-level moods.

The first state-changing operation writes the new format and keeps a `.v1.bak` copy of the original save. Status alone does not migrate the file. Old binaries cannot read the new format. Legacy totals remain historical totals and do not consume the new daily food allowance. No pet needs to be recreated.

## Privacy

**Version 0.4 changes the privacy boundary:** Git reads committed file contents locally to calculate line statistics. Devagatchi requests numeric diff statistics and paths, not patches; it filters paths then stores counts and commit metadata. Uncommitted source is not scanned. External diff helpers and text conversion are disabled, and lazy fetching is disabled so missing Git objects cause an error rather than a download.

State includes local repository paths and is not encrypted. Keep it private if those paths are sensitive. Devagatchi has no network code, analytics, or AI runtime. Git push itself can use the network independently of the pet.

## Contributing and license

Run `cargo fmt --check`, `cargo test --offline`, `cargo clippy --offline --all-targets -- -D warnings`, and `python3 scripts/check_secrets.py`. The secret scan checks common patterns in publishable files and reachable history; it is not a guarantee that every secret is detected.

Submit only code/artwork you have the right to contribute under the [MIT license](LICENSE). Devagatchi is independent and is not affiliated with or endorsed by Bandai, Bandai Namco, or OpenAI. The license does not grant rights to third-party trademarks.
