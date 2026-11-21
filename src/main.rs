use std::{
    env,
    io::IsTerminal,
    path::{Path, PathBuf},
    process::ExitCode,
};
mod activity;
mod art;
mod care;
mod git;
mod help;
mod state;
use state::{Event, Guard, Pet};
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
#[derive(Clone, Copy, PartialEq)]
enum Scope {
    Auto,
    Global,
    Repo,
    Both,
}
fn parse(args: Vec<String>) -> Result<(Vec<String>, Scope), String> {
    let mut scope = Scope::Auto;
    let mut rest = vec![];
    for arg in args {
        let selected = match arg.as_str() {
            "--global" => Some(Scope::Global),
            "--repo" => Some(Scope::Repo),
            "--both" => Some(Scope::Both),
            _ => None,
        };
        if let Some(s) = selected {
            if scope != Scope::Auto {
                return Err("Use only one of --global, --repo, or --both".into());
            }
            scope = s;
        } else {
            rest.push(arg);
        }
    }
    Ok((rest, scope))
}
fn paths(scope: Scope, mutating: bool) -> Result<Vec<(String, PathBuf)>, String> {
    let global = state::global_path()?;
    let repo = git::root(".").ok().map(|r| state::local_path(&r));
    let mut items = vec![];
    if scope != Scope::Repo && global.exists() {
        items.push(("Global".into(), global));
    }
    if scope != Scope::Global {
        if let Some(p) = repo.filter(|p| p.exists()) {
            items.push(("Repository".into(), p));
        }
    }
    if scope == Scope::Both && items.len() != 2 {
        return Err("--both requires a global and repository pet".into());
    }
    if items.is_empty() {
        return Err("No pet in this scope. Run devagatchi init NAME --global or --repo".into());
    }
    if mutating && scope == Scope::Auto && items.len() > 1 {
        return Err("Both pets exist; choose --global, --repo, or --both".into());
    }
    Ok(items)
}
fn paint(text: &str, color: &str) -> String {
    if std::io::stdout().is_terminal()
        && env::var_os("NO_COLOR").is_none_or(|v| v.is_empty())
        && env::var("TERM").unwrap_or_default() != "dumb"
    {
        if let Some(c) = state::color(color).filter(|s| !s.is_empty()) {
            return format!("\x1b[{c}m{text}\x1b[0m");
        }
    }
    text.into()
}
fn status(label: &str, pet: &Pet) {
    let v = pet.view(now());
    let mood = care::mood(v.food, &v.mode);
    let filled = (v.food / 5.).floor() as usize;
    println!("{}", paint(&art::portrait(&pet.species, mood), &pet.color));
    println!(
        "{}",
        paint(
            &format!(
                "Food [{}{}] {:.0}/100",
                "#".repeat(filled),
                "-".repeat(20 - filled),
                v.food
            ),
            &pet.color
        )
    );
    println!(
        "{} the {} · {label} pet\nMood: {} | Care mode: {}",
        pet.name,
        pet.species,
        mood.label(),
        v.mode
    );
    let seconds = (v.food * care::period(&v.mode) as f64 / 100.).ceil() as u64;
    println!(
        "Empty in: {}{}",
        care::duration(seconds),
        if v.mode == "cozy" {
            " · Cozy pets never become hungry"
        } else {
            ""
        }
    );
    println!(
        "Commits spotted: {}\nLines fed (lifetime): {}\nFood earned: {} · Legacy commit meals: {}",
        v.commits, v.lines, v.earned, pet.old_meals
    );
    println!("Connected repositories: {}", pet.repos.len());
}
fn initialize(args: &[String], scope: Scope) -> Result<(), String> {
    if !(2..=4).contains(&args.len()) || scope == Scope::Both {
        return Err(
            "Usage: devagatchi init NAME [PATH] [cozy|traditional] [--global|--repo]".into(),
        );
    }
    let name = args[1].trim();
    let mode = args.get(3).map(String::as_str).unwrap_or("cozy");
    if name.is_empty() || name.chars().any(char::is_control) || !state::valid_mode(mode) {
        return Err("Use a nonempty name and cozy or traditional mode".into());
    }
    let repo = git::binding(args.get(2).map(String::as_str).unwrap_or("."))?;
    let path = if scope == Scope::Repo {
        git::exclude_local(&repo.root)?;
        state::local_path(&repo.root)
    } else {
        state::global_path()?
    };
    let _guard = Guard::acquire(&path)?;
    if path.exists() {
        return Err("A pet already exists in that scope; it was kept".into());
    }
    let pet = Pet::new(name.into(), mode.into(), repo);
    pet.save(&path)?;
    status(
        if scope == Scope::Repo {
            "Repository"
        } else {
            "Global"
        },
        &pet,
    );
    println!("\n{}", care::GUIDE);
    Ok(())
}
fn connect() -> Result<(), String> {
    let binding = git::binding(".")?;
    let path = state::global_path()?;
    let _guard = Guard::acquire(&path)?;
    let mut pet = Pet::load(&path)?;
    if pet.repos.iter().any(|r| r.root == binding.root) {
        println!("Repository is already connected.");
        return Ok(());
    }
    pet.repos.push(binding);
    pet.save(&path)?;
    println!("Connected this repository to the global pet. Existing history is the baseline.");
    Ok(())
}
fn sync_selected(scope: Scope, catch_up: bool) -> Result<(), String> {
    let root = git::root(".").ok();
    let only = if scope == Scope::Global {
        None
    } else {
        root.as_deref()
    };
    let mut cache = activity::Cache::default();
    let mut errors = vec![];
    for (label, path) in paths(scope, false)? {
        let result = (|| -> Result<(), String> {
            let _guard = Guard::acquire(&path)?;
            let mut pet = Pet::load(&path)?;
            if only.is_some_and(|r| !pet.repos.iter().any(|b| b.root == r)) {
                return Ok(());
            }
            let before = pet.view(now()).earned;
            let report = if catch_up {
                activity::catch_up(&mut pet, only, &mut cache)?
            } else {
                activity::sync(&mut pet, only, &mut cache)?
            };
            let earned = pet.view(now()).earned.saturating_sub(before);
            pet.save(&path)?;
            println!("{label}: {} new commit(s), +{} / -{} lines, +{earned} food earned. Excess food is not banked.",report.commits,report.added,report.deleted);
            if report.pending {
                println!("More history remains; run sync --catch-up again.");
            }
            status(&label, &pet);
            Ok(())
        })();
        if let Err(e) = result {
            errors.push(format!("{label}: {e}"));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

fn run() -> Result<(), String> {
    let (mut args, scope) = parse(env::args().skip(1).collect())?;
    if args.len() == 1 && matches!(args[0].as_str(), "--version" | "-V") {
        println!("devagatchi {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args.len() == 1 && matches!(args[0].as_str(), "--help" | "-h") {
        return help::show(None);
    }
    if args.len() == 2 && matches!(args[1].as_str(), "--help" | "-h") {
        return help::show(Some(&args[0]));
    }
    if args.first().is_some_and(|s| s == "help") {
        if args.len() > 2 {
            return Err("Usage: help [command]".into());
        }
        return help::show(args.get(1).map(String::as_str));
    }
    if args.is_empty() {
        if paths(scope, false).is_err() {
            return help::show(None);
        }
        args.push("status".into());
    }
    match args[0].as_str() {
        "init" => initialize(&args, scope),
        "connect" if args.len() == 1 && matches!(scope, Scope::Auto | Scope::Global) => connect(),
        "sync" if args.len() == 1 || (args.len() == 2 && args[1] == "--catch-up") => {
            sync_selected(scope, args.len() == 2)
        }
        "reconcile" if args.len() == 1 => {
            let root = git::root(".")?;
            let head = git::head(&root)?;
            for (label, path) in paths(scope, false)? {
                let _guard = Guard::acquire(&path)?;
                let mut pet = Pet::load(&path)?;
                for repo in &mut pet.repos {
                    if repo.root == root {
                        repo.baseline = head.clone();
                        repo.head = head.clone();
                        repo.since = now();
                    }
                }
                pet.save(&path)?;
                println!("{label}: current checkout history baselined; no food awarded.");
            }
            Ok(())
        }
        "status" if args.len() == 1 => {
            for (label, path) in paths(scope, false)? {
                let _guard = Guard::acquire(&path)?;
                status(&label, &Pet::load(&path)?);
            }
            Ok(())
        }
        "feed" | "mode" | "pet" | "color" | "exclude" if args.len() == 2 => {
            let value = &args[1];
            let command = &args[0];
            let count = if command == "feed" {
                let n = value
                    .parse::<u64>()
                    .map_err(|_| "Feed a positive whole number of lines")?;
                if n == 0 {
                    return Err("Feed a positive whole number of lines".into());
                }
                n
            } else {
                0
            };
            if command == "mode" && !state::valid_mode(value)
                || command == "pet" && !state::valid_species(value)
                || command == "color" && state::color(value).is_none()
            {
                return Err("Invalid setting. Run help for that command".into());
            }
            if command == "exclude"
                && (value.is_empty()
                    || Path::new(value).is_absolute()
                    || value.split('/').any(|s| s == "..")
                    || value.chars().any(char::is_control))
            {
                return Err("Use a repository-relative file or directory prefix".into());
            }
            for (label, path) in paths(scope, true)? {
                let _guard = Guard::acquire(&path)?;
                let mut pet = Pet::load(&path)?;
                match command.as_str() {
                    "pet" => pet.species = value.clone(),
                    "color" => pet.color = value.clone(),
                    "exclude" => {
                        if !pet.excludes.contains(value) {
                            pet.excludes.push(value.clone());
                        }
                    }
                    _ => {
                        let at = now();
                        pet.events.push(Event {
                            key: format!("manual:{}:{}", at, pet.events.len()),
                            at,
                            added: count,
                            deleted: 0,
                            kind: if command == "feed" {
                                "m".into()
                            } else {
                                value.clone()
                            },
                        });
                    }
                }
                pet.save(&path)?;
                status(&label, &pet);
            }
            Ok(())
        }
        _ => Err("Unknown command or arguments. Run devagatchi --help".into()),
    }
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
