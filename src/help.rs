pub fn show(command: Option<&str>) -> Result<(), String> {
    let text = match command {
        None => r"DEVAGATCHI - your offline terminal companion

USAGE
  devagatchi <command> [arguments]
  devagatchi                      Show your pet (or help before setup)

COMMANDS
  init <name> [repo] [mode]       Adopt a cat; defaults: current folder, cozy
  status                         Show pet, mood, and lifetime totals
  sync                           Feed one meal per new commit and show status
  feed <lines>                   Feed a manually entered positive line count
  pet <species>                  Choose cat, dog, bird, or lizard
  color <color>                  Choose cyan, green, yellow, blue, magenta,
                                 red, white, or off (default: cyan)
  mode <mode>                    Choose cozy or traditional care
  help [command]                 Show help for all commands or one command

OPTIONS
  -h, --help                     Show help (also after any command)
  -V, --version                  Show installed version

EXAMPLES
  devagatchi init Mochi . cozy
  devagatchi color magenta
  devagatchi pet lizard
  devagatchi sync
  devagatchi feed 42
  devagatchi feed --help

PRIVACY & FEEDING
  Git metadata counts commits only. No source contents or diffs are requested.
  Sync feeds one commit meal per newly detected commit and refreshes care.
  Commit meals are separate from manually entered lines. For an extra meal:
  `devagatchi feed 42` feeds 42 lines. Replace placeholders; do not type < or >.
  Run sync after committing: there is no background watcher or network connection.
  Color is disabled for redirected output, TERM=dumb, or a nonempty NO_COLOR.

CARE
  Cozy: sleepy after 72 hours. Traditional: hungry after 24, very hungry after 48.
  Feeding refreshes care in either mode. Neither mode causes permanent harm.

STORAGE
  One pet per data directory. Existing pets survive reinstalls and updates.
  $XDG_DATA_HOME/devagatchi/pet.txt, or ~/.local/share/devagatchi/pet.txt.",
        Some("init") => "USAGE: devagatchi init <name> [repo] [cozy|traditional]\n\nAdopt a cat. Repository defaults to the current folder; care defaults to cozy.\nExisting commits are the baseline. An existing pet will never be overwritten.\nQuote names and paths that contain spaces.\n\nExample: devagatchi init Mochi . cozy",
        Some("status") => "USAGE: devagatchi status\n\nShow your pet, care mode, mood, lifetime commits, and lifetime lines fed.\nDoes not scan Git or feed your pet. Running devagatchi alone also shows status.",
        Some("sync") => "USAGE: devagatchi sync\n\nFeed one commit meal per new commit since the last sync and refresh care.\nCommit meals do not change your manually entered line total.\nRepeat syncs with no new commits do not feed or refresh care.\nUnrelated or rewritten history is rejected without changing progress.\nPreviously tracked commits are not retroactively fed. Run this command after\ncommitting; there is no background watcher or Git hook.",
        Some("color") => "USAGE: devagatchi color <cyan|green|yellow|blue|magenta|red|white|off>\n\nSave a color for the portrait and mood (default: cyan).\nColors appear only in an interactive terminal. Redirected output, TERM=dumb,\nand a nonempty NO_COLOR disable color. Example: devagatchi color green",
        Some("feed") => "USAGE: devagatchi feed <lines>\n\nReplace <lines> with a positive whole number, without angle brackets.\nThe number is entered manually: source contents are never inspected.\nShows this meal's size and updated lifetime totals, and refreshes care.\n\nExample: devagatchi feed 42",
        Some("pet") => "USAGE: devagatchi pet <cat|dog|bird|lizard>\n\nChange your pet's appearance and show status. Name and progress are preserved.\n\nExample: devagatchi pet bird",
        Some("mode") => "USAGE: devagatchi mode <cozy|traditional>\n\nCozy: sleepy after 72 hours without food.\nTraditional: hungry after 24 hours, very hungry after 48.\nNeither mode causes permanent harm. Changing mode does not reset feeding time.\n\nExample: devagatchi mode cozy",
        Some("help") => "USAGE: devagatchi help [command]\n\nExamples: devagatchi --help, devagatchi help feed, devagatchi feed --help",
        Some(other) => return Err(format!("Unknown command '{other}'. Run `devagatchi --help`.")),
    };
    println!("{text}");
    Ok(())
}
