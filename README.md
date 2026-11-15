# Devagatchi 🐾

An offline terminal pet for developers. Track commits, feed your pet lines of code, and choose how much care it needs. Devagatchi never reads your source files, and it has no AI, accounts, network features, or third-party Rust dependencies.

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

2. Build Devagatchi:

   ```sh
   cargo build --release --offline
   ```

3. Create your pet. Replace `Mochi` with its name and `/path/to/your/repository` with the local Git repository you want to track. Keep the quotes if the path has spaces. You can also use `.` to track the repository you're currently in.

   ```sh
   ./target/release/devagatchi init Mochi "/path/to/your/repository" cozy
   ```

   Choose `cozy` or `traditional`. Your pet starts as a cat; you can change it below. Existing commits establish the starting point and are not counted as new activity.

4. Check in and choose a pet:

   ```sh
   ./target/release/devagatchi status
   ./target/release/devagatchi pet dog
   ```

   Available pets: `cat`, `dog`, `bird`, and `lizard`. You can change your pet's appearance at any time without losing progress.

5. After making a commit, check for new activity and feed your pet:

   ```sh
   ./target/release/devagatchi sync
   ./target/release/devagatchi feed 42
   ```

   Replace `42` with the number of lines you want to feed it. Feeding shows your pet's updated status. Line counts are entered manually; calculating additions or deletions from Git would require reading file contents.

Run these commands from the Devagatchi project directory, or use the full path to the executable. To install the executable for easier access from any directory, run `cargo install --path . --offline`, then use `devagatchi status` and the other commands without the `./target/release/` prefix. Cargo's binary directory must be on your `PATH`.

## Care and commands

Cozy pets become sleepy after three days without food. Traditional pets become hungry after one day and very hungry after two. Neither mode currently causes permanent harm. Switch anytime with `devagatchi mode cozy` or `devagatchi mode traditional` (use the executable path shown above if you haven't installed it). Run `devagatchi --help` for the command list.

`sync` runs only when you ask it to. It counts new commits reachable from the tracked HEAD, including merged commits. If history is rewritten or you switch to unrelated history, sync stops instead of guessing or double-counting. For an empty repository, commits are counted once they become reachable from HEAD.

## Privacy

Devagatchi runs locally and has no network code. It calls Git only for repository validation, commit IDs, ancestry, and commit counts; it does not request diffs, file names, commit messages, or source contents. Git itself reads repository metadata and commit objects for these operations. The selected repository path, last observed commit ID, pet settings, and progress are stored in `$XDG_DATA_HOME/devagatchi/pet.txt`, or `~/.local/share/devagatchi/pet.txt` when `XDG_DATA_HOME` is unset. Treat that file as private if your repository path is sensitive.

This is an early terminal prototype. Desktop and optional editor integrations are future possibilities.

## Project independence

Devagatchi is an independent virtual-pet project. It is not affiliated with, endorsed by, or an official product of Bandai, Bandai Namco, or OpenAI. Other companies' product names and trademarks belong to their respective owners.

## Contributing

Contributions and ideas are welcome. Please submit only code, artwork, and other material you have the right to contribute under the project's [MIT license](LICENSE). Do not copy other projects' characters, artwork, logos, or code without the necessary permission and license compliance. Run `cargo fmt --check` and `cargo test --offline` before submitting changes.

## License

Devagatchi is available under the [MIT license](LICENSE). The license applies to this project's contents and does not grant rights to third-party trademarks.
