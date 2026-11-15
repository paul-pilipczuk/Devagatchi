use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
struct Pet {
    name: String,
    mode: String,
    species: String,
    repo: String,
    head: String,
    commits: u64,
    lines: u64,
    last_fed: u64,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn data_file() -> Result<PathBuf, String> {
    let base = match env::var_os("XDG_DATA_HOME") {
        Some(p) if !p.is_empty() => PathBuf::from(p),
        _ => PathBuf::from(env::var_os("HOME").ok_or("HOME is not set; set XDG_DATA_HOME")?)
            .join(".local/share"),
    };
    Ok(base.join("devagatchi/pet.txt"))
}

// Hex encoding keeps the small, dependency-free state format unambiguous.
fn encode(s: &str) -> String {
    s.as_bytes().iter().map(|b| format!("{b:02x}")).collect()
}
fn decode(s: &str) -> Result<String, String> {
    if s.len() % 2 != 0 || !s.is_ascii() {
        return Err("Invalid state encoding".into());
    }
    let bytes = (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| "Invalid state encoding".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    String::from_utf8(bytes).map_err(|_| "Invalid state encoding".into())
}
impl Pet {
    fn load(path: &Path) -> Result<Self, String> {
        let data = fs::read_to_string(path)
            .map_err(|e| format!("Could not read pet state: {e}. Run `devagatchi init` first."))?;
        let parts: Vec<_> = data.lines().collect();
        if !(parts.len() == 8 || parts.len() == 9) || parts[0] != "devagatchi-v1" {
            return Err("Unrecognized or damaged pet state".into());
        }
        let number = |s: &str| {
            s.parse::<u64>()
                .map_err(|_| "Invalid number in pet state".to_string())
        };
        let pet = Self {
            name: decode(parts[1])?,
            mode: parts[2].into(),
            species: parts.get(8).unwrap_or(&"cat").to_string(),
            repo: decode(parts[3])?,
            head: parts[4].into(),
            commits: number(parts[5])?,
            lines: number(parts[6])?,
            last_fed: number(parts[7])?,
        };
        if !matches!(pet.mode.as_str(), "cozy" | "traditional")
            || !valid_species(&pet.species)
            || (!pet.head.is_empty() && !pet.head.bytes().all(|b| b.is_ascii_hexdigit()))
        {
            return Err("Invalid pet state".into());
        }
        Ok(pet)
    }
    fn save(&self, path: &Path) -> Result<(), String> {
        let dir = path.parent().ok_or("Invalid state path")?;
        fs::create_dir_all(dir).map_err(|e| format!("Could not create data directory: {e}"))?;
        let data = format!(
            "devagatchi-v1\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n",
            encode(&self.name),
            self.mode,
            encode(&self.repo),
            self.head,
            self.commits,
            self.lines,
            self.last_fed,
            self.species
        );
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, data).map_err(|e| format!("Could not write pet state: {e}"))?;
        fs::rename(tmp, path).map_err(|e| format!("Could not save pet state: {e}"))
    }
    fn mood(&self, time: u64) -> &'static str {
        let hours = time.saturating_sub(self.last_fed) / 3600;
        if self.mode == "cozy" {
            if hours >= 72 {
                "sleepy"
            } else {
                "content"
            }
        } else if hours >= 48 {
            "very hungry"
        } else if hours >= 24 {
            "hungry"
        } else {
            "happy"
        }
    }
}

fn valid_species(species: &str) -> bool {
    matches!(species, "cat" | "dog" | "bird" | "lizard")
}

fn show_status(pet: &Pet) {
    let art = match pet.species.as_str() {
        "dog" => {
            r" / \__
(    @\___
 /         O
/   (_____/
/_____/   U"
        }
        "bird" => {
            r"  \\  //
 ( o  o )
  \\__//
   /  \"
        }
        "lizard" => {
            r"     __
  .-'  '-._
 /  o    o  \__
(      ^       _>
 \_  ----  _.-'"
        }
        _ => {
            r"  /\_/\
 ( o.o )
  > ^ <"
        }
    };
    println!(
        "{art}\n\n{} the {} is {} ({})\nCommits spotted: {}\nLines fed: {}\nRepository: {}",
        pet.name,
        pet.species,
        pet.mood(now()),
        pet.mode,
        pet.commits,
        pet.lines,
        pet.repo
    );
}

// Git is only asked for commit IDs and ancestry/count metadata, never diffs or file contents.
fn git(repo: &str, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|e| format!("Could not run Git: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "Git failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    String::from_utf8(out.stdout)
        .map(|s| s.trim().to_owned())
        .map_err(|_| "Git returned invalid UTF-8".into())
}
fn head(repo: &str) -> Result<String, String> {
    // An unborn branch has no HEAD. Check that the repository itself exists first.
    git(repo, &["rev-parse", "--git-dir"])?;
    match git(repo, &["rev-parse", "--verify", "HEAD^{commit}"]) {
        Ok(id) => Ok(id),
        Err(_) => {
            if git(repo, &["rev-list", "--all", "--max-count=1"])?.is_empty() {
                Ok(String::new())
            } else {
                Err("Cannot resolve HEAD; check the repository".into())
            }
        }
    }
}
fn usage() {
    println!("Devagatchi — a tiny offline developer pet\n\n  devagatchi init <name> <repo> [cozy|traditional]\n  devagatchi status\n  devagatchi sync\n  devagatchi feed <lines>\n  devagatchi mode <cozy|traditional>\n  devagatchi pet <cat|dog|bird|lizard>\n\nNo source code is read to track commits. Line counts are entered manually.");
}
fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || matches!(args[0].as_str(), "help" | "--help" | "-h") {
        usage();
        return Ok(());
    }
    let path = data_file()?;
    match args[0].as_str() {
        "init" => {
            if !(args.len() == 3 || args.len() == 4) {
                return Err("Usage: devagatchi init <name> <repo> [cozy|traditional]".into());
            }
            if path.exists() {
                return Err("A pet already exists. Your existing pet was kept.".into());
            }
            let mode = args.get(3).map(String::as_str).unwrap_or("cozy");
            if !matches!(mode, "cozy" | "traditional") {
                return Err("Mode must be cozy or traditional".into());
            }
            if args[1].trim().is_empty() {
                return Err("Pet name cannot be empty".into());
            }
            let repo =
                fs::canonicalize(&args[2]).map_err(|e| format!("Cannot access repository: {e}"))?;
            let repo = repo
                .to_str()
                .ok_or("Repository path must be valid UTF-8")?
                .to_owned();
            let pet = Pet {
                name: args[1].clone(),
                mode: mode.into(),
                species: "cat".into(),
                head: head(&repo)?,
                repo,
                commits: 0,
                lines: 0,
                last_fed: now(),
            };
            pet.save(&path)?;
            println!(
                "Welcome, {}! Your pet is in {} mode. Existing commits are the starting point.",
                pet.name, pet.mode
            );
        }
        "status" if args.len() == 1 => {
            let pet = Pet::load(&path)?;
            show_status(&pet);
        }
        "feed" if args.len() == 2 => {
            let count: u64 = args[1]
                .parse()
                .map_err(|_| "Enter a positive whole number of lines")?;
            if count == 0 {
                return Err("Enter a positive whole number of lines".into());
            }
            let mut pet = Pet::load(&path)?;
            pet.lines = pet
                .lines
                .checked_add(count)
                .ok_or("Line total is too large")?;
            pet.last_fed = now();
            pet.save(&path)?;
            println!("{} happily ate {count} lines!\n", pet.name);
            show_status(&pet);
        }
        "pet" if args.len() == 2 => {
            if !valid_species(&args[1]) {
                return Err("Pet must be cat, dog, bird, or lizard".into());
            }
            let mut pet = Pet::load(&path)?;
            pet.species = args[1].clone();
            pet.save(&path)?;
            show_status(&pet);
        }
        "mode" if args.len() == 2 => {
            if !matches!(args[1].as_str(), "cozy" | "traditional") {
                return Err("Mode must be cozy or traditional".into());
            }
            let mut pet = Pet::load(&path)?;
            pet.mode = args[1].clone();
            pet.save(&path)?;
            println!("{} is now in {} mode.", pet.name, pet.mode);
        }
        "sync" if args.len() == 1 => {
            let mut pet = Pet::load(&path)?;
            let new_head = head(&pet.repo)?;
            if new_head == pet.head {
                println!("No new commits for {}.", pet.name);
                return Ok(());
            }
            if new_head.is_empty() {
                return Err("HEAD has disappeared; no progress was changed.".into());
            }
            let count = if pet.head.is_empty() {
                git(&pet.repo, &["rev-list", "--count", &new_head])?
            } else {
                git(&pet.repo, &["merge-base", "--is-ancestor", &pet.head, &new_head]).map_err(|_| "The tracked history was rewritten or changed branches; no progress was changed. Return to the tracked history to sync.".to_string())?;
                git(
                    &pet.repo,
                    &[
                        "rev-list",
                        "--count",
                        &format!("{}..{}", pet.head, new_head),
                    ],
                )?
            };
            let count: u64 = count.parse().map_err(|_| "Invalid commit count from Git")?;
            pet.commits = pet
                .commits
                .checked_add(count)
                .ok_or("Commit total is too large")?;
            pet.head = new_head;
            pet.save(&path)?;
            println!("{} spotted {count} new commit(s)! Feed lines manually with `devagatchi feed <lines>`.", pet.name);
        }
        _ => return Err("Unknown command or arguments. Run `devagatchi --help`.".into()),
    }
    Ok(())
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encoding_round_trip() {
        let s = "Mochi 🐈\n%";
        assert_eq!(decode(&encode(s)).unwrap(), s);
        assert!(decode("0x").is_err());
    }
    #[test]
    fn care_modes() {
        let mut pet = Pet {
            name: "M".into(),
            mode: "cozy".into(),
            species: "cat".into(),
            repo: "".into(),
            head: "".into(),
            commits: 0,
            lines: 0,
            last_fed: 100,
        };
        assert_eq!(pet.mood(100 + 72 * 3600), "sleepy");
        pet.mode = "traditional".into();
        assert_eq!(pet.mood(100 + 24 * 3600), "hungry");
        assert_eq!(pet.mood(100 + 48 * 3600), "very hungry");
    }
}
