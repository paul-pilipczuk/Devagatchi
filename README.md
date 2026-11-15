# Devagatchi 🐾

An offline terminal pet for developers. Feed your pet with commits or manually entered lines of code, and choose how much care it needs. Devagatchi never reads your source files, and it has no AI, accounts, network features, or third-party Rust dependencies.

## What you need

- A terminal on macOS or Linux.
- Git and Rust's Cargo installed. Run `git --version` and `cargo --version` to check. If Cargo is missing, follow the [official Rust installation instructions](https://www.rust-lang.org/tools/install); installing Rust requires an internet connection. Once built, Devagatchi runs offline.
- A local Git repository you want your pet to watch.

## Get started

1. Clone this repository and enter its folder:

   ```sh
   git clone https://github.com/paul-pilipczuk/Devagatchi.git
   cd Devagatchi
   ```

   If you already have the project folder, just open a terminal and `cd` into it.

2. Install Devagatchi as a terminal command:

   ```sh
   cargo install --path . --offline --force
   ```

3. Create your pet. Replace `Mochi` with its name and `/path/to/your/repository` with the local Git repository you want to track. Keep the quotes if the path has spaces. You can also use `.` to track the repository you're currently in.

   ```sh
   devagatchi init Mochi "/path/to/your/repository" cozy
   ```

   Choose `cozy` or `traditional`. Your pet starts as a cat; you can change it below. Existing commits establish the starting point and are not counted as new activity.

4. Check in and choose a pet:

   ```sh
   devagatchi status
   devagatchi pet dog
   ```

   Available pets: `cat`, `dog`, `bird`, and `lizard`. You can change your pet's appearance at any time without losing progress.

5. After making a commit, feed your pet:

   ```sh
   devagatchi sync
   ```

   Each newly detected commit serves one commit meal and refreshes care. Status shows meals and manual lines separately. You can also feed an extra meal with `devagatchi feed 42`, replacing `42` with your own line count. No source contents are inspected.

After installation, `devagatchi` works from any directory. Running it alone shows your pet (or setup help if you have not adopted one yet). You can also adopt a pet with `devagatchi init Mochi` to track the current directory in cozy mode.

If your terminal says `command not found`, load Cargo's environment with `source "$HOME/.cargo/env"` and try again. On a standard Rust installation, restarting your terminal also loads this configuration. Custom Cargo installations should add their own `bin` directory to `PATH`.

To update an existing installation, run `git pull` followed by `cargo install --path . --offline --force` from the project folder. Your pet's progress is stored separately and survives installation. You do not need to run `init` again.

## Command reference

| Command | What it does |
| --- | --- |
| `devagatchi` or `devagatchi status` | Show your pet, mood, and lifetime totals |
| `devagatchi init Mochi . cozy` | Adopt a pet and track the current repository |
| `devagatchi sync` | Feed one meal per new commit and show status |
| `devagatchi feed 42` | Feed 42 manually entered lines and show updated status |
| `devagatchi pet dog` | Choose `cat`, `dog`, `bird`, or `lizard` |
| `devagatchi color magenta` | Choose a saved portrait and mood color |
| `devagatchi mode cozy` | Choose `cozy` or `traditional` care |
| `devagatchi --help` | Show all commands, options, care rules, and storage details |
| `devagatchi feed --help` | Show help for a specific command |
| `devagatchi help sync` | Another way to get command-specific help |
| `devagatchi --version` | Show the installed version |

### Automatic feeding during sync

Run `devagatchi sync` after committing. Each newly detected commit feeds one commit meal and refreshes the care timer. Repeating a sync without new commits does not serve more meals or refresh the timer. Commits counted before this update are not retroactively fed. Initial setup still uses existing commits as the baseline.

The app does not run in the background or install Git hooks: committing alone does not trigger it. Commit meals are counted separately from manually entered lines; Git metadata cannot reveal line counts without inspecting source contents. `devagatchi feed 42` remains available for a manual meal.

### Terminal colors

Use `devagatchi color magenta` to save your preferred color. Options: `cyan` (default), `green`, `yellow`, `blue`, `magenta`, `red`, `white`, and `off`. Colors apply to the portrait and mood. Redirected output, `TERM=dumb`, or a nonempty `NO_COLOR` environment variable disables ANSI colors. Example: `NO_COLOR=1 devagatchi status`.

## Care and commands

Cozy pets become sleepy after three days without food. Traditional pets become hungry after one day and very hungry after two. Neither mode currently causes permanent harm. Switch anytime with `devagatchi mode cozy` or `devagatchi mode traditional`. Run `devagatchi --help` for the command list.

`sync` runs only when you ask it to. It counts new commits reachable from the tracked HEAD, including merged commits. If history is rewritten or you switch to unrelated history, sync stops instead of guessing or double-counting. For an empty repository, commits are counted once they become reachable from HEAD.

## Privacy

Devagatchi runs locally and has no network code. It calls Git only for repository validation, commit IDs, ancestry, and commit counts; it does not request diffs, file names, commit messages, or source contents. Git itself reads repository metadata and commit objects for these operations. The selected repository path, last observed commit ID, pet settings, and progress are stored in `$XDG_DATA_HOME/devagatchi/pet.txt`, or `~/.local/share/devagatchi/pet.txt` when `XDG_DATA_HOME` is unset. Treat that file as private if your repository path is sensitive.

This is an early terminal prototype. Desktop and optional editor integrations are future possibilities.

## Project independence

Devagatchi is an independent virtual-pet project. It is not affiliated with, endorsed by, or an official product of Bandai, Bandai Namco, or OpenAI. Other companies' product names and trademarks belong to their respective owners.

## Contributing

Contributions and ideas are welcome. Please submit only code, artwork, and other material you have the right to contribute under the project's [MIT license](LICENSE). Do not copy other projects' characters, artwork, logos, or code without the necessary permission and license compliance. Run `cargo fmt --check`, `cargo test --offline`, and `python3 scripts/check_secrets.py` before submitting changes. The local secret check scans publishable files and reachable Git history for common credential patterns; it is a precaution, not a guarantee that every secret is detected.

## License

Devagatchi is available under the [MIT license](LICENSE). The license applies to this project's contents and does not grant rights to third-party trademarks.
