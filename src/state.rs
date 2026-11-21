use crate::{care, git, now};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct Binding {
    pub root: String,
    pub common: String,
    pub baseline: String,
    pub head: String,
    pub since: u64,
}
#[derive(Clone, Debug)]
pub struct Event {
    pub key: String,
    pub at: u64,
    pub added: u64,
    pub deleted: u64,
    // c: commit; m: manual; cozy/traditional: mode change
    pub kind: String,
}
#[derive(Clone, Debug)]
pub struct Pet {
    pub name: String,
    pub species: String,
    pub color: String,
    pub start: u64,
    pub initial_food: f64,
    pub initial_mode: String,
    pub old_commits: u64,
    pub old_meals: u64,
    pub old_lines: u64,
    pub repos: Vec<Binding>,
    pub events: Vec<Event>,
    pub seen: BTreeSet<String>,
    pub excludes: Vec<String>,
}
pub struct View {
    pub food: f64,
    pub mode: String,
    pub earned: u64,
    pub commits: u64,
    pub lines: u64,
}
pub fn encode(s: &str) -> String {
    s.bytes().map(|b| format!("{b:02x}")).collect()
}
pub fn decode(s: &str) -> Result<String, String> {
    if !s.len().is_multiple_of(2) || !s.is_ascii() {
        return Err("Invalid state encoding".into());
    }
    String::from_utf8(
        (0..s.len())
            .step_by(2)
            .map(|i| {
                u8::from_str_radix(&s[i..i + 2], 16)
                    .map_err(|_| "Invalid state encoding".to_string())
            })
            .collect::<Result<Vec<_>, _>>()?,
    )
    .map_err(|_| "Invalid UTF-8 state".into())
}
fn number(s: &str) -> Result<u64, String> {
    s.parse().map_err(|_| "Invalid number in pet state".into())
}
pub fn valid_mode(s: &str) -> bool {
    matches!(s, "cozy" | "traditional")
}
pub fn valid_species(s: &str) -> bool {
    matches!(s, "cat" | "dog" | "bird" | "lizard")
}
pub fn color(s: &str) -> Option<&'static str> {
    match s {
        "cyan" => Some("36"),
        "green" => Some("32"),
        "yellow" => Some("33"),
        "blue" => Some("34"),
        "magenta" => Some("35"),
        "red" => Some("31"),
        "white" => Some("37"),
        "off" => Some(""),
        _ => None,
    }
}
pub fn global_path() -> Result<PathBuf, String> {
    let root = std::env::var_os("XDG_DATA_HOME")
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|s| PathBuf::from(s).join(".local/share")))
        .ok_or("Set HOME or XDG_DATA_HOME")?;
    if !root.is_absolute() {
        return Err("Data directory must be an absolute path".into());
    }
    Ok(root.join("devagatchi/pet.txt"))
}
pub fn local_path(root: &str) -> PathBuf {
    Path::new(root).join(".devagatchi.local")
}
fn reject_symlink(path: &Path) -> Result<(), String> {
    if fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(format!("Refusing symlink: {}", path.display()));
    }
    Ok(())
}
// OS-owned advisory locks release even if the hook times out or the process crashes.
// Do not delete lock files: replacing the inode would break concurrent locking.
pub struct Guard {
    _file: File,
}
impl Guard {
    pub fn acquire(path: &Path) -> Result<Self, String> {
        let parent = path.parent().ok_or("Invalid state path")?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        reject_symlink(path)?;
        let lock = PathBuf::from(format!("{}.lock", path.display()));
        reject_symlink(&lock)?;
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options.open(lock).map_err(|e| e.to_string())?;
        file.try_lock()
            .map_err(|_| "Pet is busy; retry sync shortly".to_string())?;
        Ok(Self { _file: file })
    }
}
impl Pet {
    pub fn new(name: String, mode: String, repo: Binding) -> Self {
        Self {
            name,
            species: "cat".into(),
            color: "cyan".into(),
            start: now(),
            initial_food: 100.,
            initial_mode: mode,
            old_commits: 0,
            old_meals: 0,
            old_lines: 0,
            repos: vec![repo],
            events: vec![],
            seen: BTreeSet::new(),
            excludes: vec![],
        }
    }
    pub fn view(&self, time: u64) -> View {
        let mut food = self.initial_food;
        let mut mode = self.initial_mode.clone();
        let mut at = self.start;
        let mut days = BTreeMap::<u64, u64>::new();
        let mut earned = 0u64;
        let mut commits = self.old_commits;
        let mut lines = self.old_lines;
        let mut events: Vec<_> = self.events.iter().collect();
        events.sort_by(|a, b| (a.at, &a.key).cmp(&(b.at, &b.key)));
        for e in events {
            let when = e.at.max(self.start).min(time.max(self.start));
            food = care::drain(food, &mode, when.saturating_sub(at));
            at = at.max(when);
            if valid_mode(&e.kind) {
                mode = e.kind.clone();
                continue;
            }
            if e.kind == "c" {
                commits = commits.saturating_add(1);
            }
            let count = e.added.saturating_add(e.deleted);
            lines = lines.saturating_add(count);
            let daily = days.entry(e.at / 86400).or_default();
            let before = care::earned(*daily);
            *daily = daily.saturating_add(count);
            let meal = care::earned(*daily) - before;
            earned = earned.saturating_add(meal);
            food = (food + meal as f64).min(100.);
        }
        food = care::drain(food, &mode, time.saturating_sub(at));
        View {
            food,
            mode,
            earned,
            commits,
            lines,
        }
    }
    pub fn load(path: &Path) -> Result<Self, String> {
        reject_symlink(path)?;
        let data = fs::read_to_string(path).map_err(|e| {
            format!(
                "Cannot read {}: {e}. Run devagatchi init first.",
                path.display()
            )
        })?;
        let rows: Vec<_> = data.lines().collect();
        if rows.first() == Some(&"devagatchi-v1") {
            return Self::legacy(&rows);
        }
        if rows.len() < 3 || rows[0] != "devagatchi-v2" {
            return Err("Unrecognized pet state".into());
        }
        let fields: Vec<_> = rows[1].split('\t').collect();
        let base: Vec<_> = rows[2].split('\t').collect();
        if fields.len() != 3 || base.len() != 6 {
            return Err("Damaged pet state".into());
        }
        let mut pet = Self {
            name: decode(fields[0])?,
            species: fields[1].into(),
            color: fields[2].into(),
            start: number(base[0])?,
            initial_food: base[1].parse::<f64>().map_err(|_| "Invalid food")?,
            initial_mode: base[2].into(),
            old_commits: number(base[3])?,
            old_meals: number(base[4])?,
            old_lines: number(base[5])?,
            repos: vec![],
            events: vec![],
            seen: BTreeSet::new(),
            excludes: vec![],
        };
        for row in &rows[3..] {
            let f: Vec<_> = row.split('\t').collect();
            match f.as_slice() {
                ["R", root, common, baseline, head, since] => {
                    if !git::oid_or_empty(baseline) || !git::oid_or_empty(head) {
                        return Err("Invalid checkpoint".into());
                    }
                    pet.repos.push(Binding {
                        root: decode(root)?,
                        common: decode(common)?,
                        baseline: (*baseline).into(),
                        head: (*head).into(),
                        since: number(since)?,
                    });
                }
                ["E", key, at, added, deleted, kind]
                    if matches!(*kind, "c" | "m" | "cozy" | "traditional") =>
                {
                    pet.events.push(Event {
                        key: decode(key)?,
                        at: number(at)?,
                        added: number(added)?,
                        deleted: number(deleted)?,
                        kind: (*kind).into(),
                    })
                }
                ["S", key] => {
                    pet.seen.insert(decode(key)?);
                }
                ["X", prefix] => pet.excludes.push(decode(prefix)?),
                _ => return Err("Damaged state record".into()),
            }
        }
        pet.validate()?;
        Ok(pet)
    }
    fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty()
            || self.name.chars().any(char::is_control)
            || !valid_species(&self.species)
            || color(&self.color).is_none()
            || !valid_mode(&self.initial_mode)
            || !self.initial_food.is_finite()
            || !(0.0..=100.).contains(&self.initial_food)
        {
            return Err("Invalid pet settings".into());
        }
        Ok(())
    }
    fn legacy(p: &[&str]) -> Result<Self, String> {
        if !matches!(p.len(), 8 | 9 | 11) {
            return Err("Damaged legacy state".into());
        }
        let root = git::root(&decode(p[3])?)?;
        let timestamp = number(p[7])?;
        let binding = Binding {
            common: git::common(&root)?,
            root,
            baseline: p[4].into(),
            head: p[4].into(),
            since: now(),
        };
        let mut pet = Self::new(decode(p[1])?, p[2].into(), binding);
        pet.species = p.get(8).unwrap_or(&"cat").to_string();
        pet.color = p.get(10).unwrap_or(&"cyan").to_string();
        pet.old_commits = number(p[5])?;
        pet.old_lines = number(p[6])?;
        pet.old_meals = p.get(9).map(|s| number(s)).transpose()?.unwrap_or(0);
        pet.initial_food = care::drain(100., p[2], now().saturating_sub(timestamp));
        pet.validate()?;
        Ok(pet)
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        reject_symlink(path)?;
        let mut data = format!(
            "devagatchi-v2\n{}\t{}\t{}\n{}\t{}\t{}\t{}\t{}\t{}\n",
            encode(&self.name),
            self.species,
            self.color,
            self.start,
            self.initial_food,
            self.initial_mode,
            self.old_commits,
            self.old_meals,
            self.old_lines
        );
        for r in &self.repos {
            data += &format!(
                "R\t{}\t{}\t{}\t{}\t{}\n",
                encode(&r.root),
                encode(&r.common),
                r.baseline,
                r.head,
                r.since
            );
        }
        for e in &self.events {
            data += &format!(
                "E\t{}\t{}\t{}\t{}\t{}\n",
                encode(&e.key),
                e.at,
                e.added,
                e.deleted,
                e.kind
            );
        }
        for s in &self.seen {
            data += &format!("S\t{}\n", encode(s));
        }
        for x in &self.excludes {
            data += &format!("X\t{}\n", encode(x));
        }
        let temp = PathBuf::from(format!("{}.tmp", path.display()));
        reject_symlink(&temp)?;
        let mut options = OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temp).map_err(|e| e.to_string())?;
        file.write_all(data.as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        fs::rename(temp, path).map_err(|e| e.to_string())
    }
}
