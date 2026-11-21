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
    pub files: u64,
    // b/u: balanced commit/manual; c/m: legacy commit/manual; n: prior bonus commit.
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
        let mut days = BTreeMap::<u64, (u64, u64, u64, u64, u64)>::new();
        let mut earned = 0u64;
        let mut commits = self.old_commits;
        let mut lines = self.old_lines;
        let mut events: Vec<_> = self.events.iter().collect();
        // Stable ordering preserves consecutive settings changes in the same second.
        events.sort_by_key(|e| e.at);
        for e in events {
            let when = e.at.max(self.start).min(time.max(self.start));
            food = care::drain(food, &mode, when.saturating_sub(at));
            at = at.max(when);
            if valid_mode(&e.kind) {
                mode = e.kind.clone();
                continue;
            }
            if matches!(e.kind.as_str(), "c" | "n" | "b") {
                commits = commits.saturating_add(1);
            }
            let count = e.added.saturating_add(e.deleted);
            lines = lines.saturating_add(count);
            let daily = days.entry(e.at / 86400).or_default();
            let old_lines = daily.0;
            daily.0 = daily.0.saturating_add(count);
            let reward = if matches!(e.kind.as_str(), "b" | "u") {
                let line_reward = care::balanced_lines(daily.0) - care::balanced_lines(old_lines);
                let old_commit = daily.2.min(5) * 5;
                let old_files = daily.3.min(5) * 3;
                if e.kind == "b" {
                    daily.2 = daily.2.saturating_add(1);
                    daily.3 = daily.3.saturating_add(e.files);
                }
                line_reward + daily.2.min(5) * 5 - old_commit + daily.3.min(5) * 3 - old_files
            } else {
                let before = care::earned(old_lines).saturating_add(daily.1).min(100);
                if e.kind == "n" {
                    daily.1 = daily
                        .1
                        .saturating_add(2)
                        .saturating_add(e.files.saturating_mul(5));
                }
                care::earned(daily.0).saturating_add(daily.1).min(100) - before
            };
            let meal = reward.min(100 - daily.4);
            daily.4 += meal;
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
                    if matches!(*kind, "c" | "m" | "u" | "cozy" | "traditional") =>
                {
                    pet.events.push(Event {
                        key: decode(key)?,
                        at: number(at)?,
                        added: number(added)?,
                        deleted: number(deleted)?,
                        files: 0,
                        kind: (*kind).into(),
                    })
                }
                ["E", key, at, added, deleted, kind @ ("n" | "b"), files] => {
                    pet.events.push(Event {
                        key: decode(key)?,
                        at: number(at)?,
                        added: number(added)?,
                        deleted: number(deleted)?,
                        files: number(files)?,
                        kind: (*kind).into(),
                    });
                }
                ["S", key] => {
                    pet.seen.insert(decode(key)?);
                }
                ["X", prefix] => pet.excludes.push(decode(prefix)?),
                _ => return Err("Damaged state record".into()),
            }
        }
        // Activity records also establish identity; a missing auxiliary S row cannot
        // make an already-recorded commit eligible for another meal.
        for event in &pet.events {
            if matches!(event.kind.as_str(), "c" | "n" | "b") {
                pet.seen.insert(event.key.clone());
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
        let mut keys = BTreeSet::new();
        if self.events.iter().any(|e| !keys.insert(&e.key)) {
            return Err("Duplicated activity record".into());
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
                "E\t{}\t{}\t{}\t{}\t{}{}\n",
                encode(&e.key),
                e.at,
                e.added,
                e.deleted,
                e.kind,
                if matches!(e.kind.as_str(), "n" | "b") {
                    format!("\t{}", e.files)
                } else {
                    String::new()
                }
            );
        }
        for s in &self.seen {
            data += &format!("S\t{}\n", encode(s));
        }
        for x in &self.excludes {
            data += &format!("X\t{}\n", encode(x));
        }
        // Keep the original v1 save before the first v2 write. Never overwrite a backup.
        if fs::read_to_string(path).is_ok_and(|s| s.starts_with("devagatchi-v1\n")) {
            let backup = PathBuf::from(format!("{}.v1.bak", path.display()));
            reject_symlink(&backup)?;
            if !backup.exists() {
                let original = fs::read(path).map_err(|e| e.to_string())?;
                let mut options = OpenOptions::new();
                options.write(true).create_new(true);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::OpenOptionsExt;
                    options.mode(0o600);
                }
                let mut file = options.open(backup).map_err(|e| e.to_string())?;
                file.write_all(&original)
                    .and_then(|_| file.sync_all())
                    .map_err(|e| e.to_string())?;
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    fn pet() -> Pet {
        Pet {
            name: "Test".into(),
            species: "cat".into(),
            color: "off".into(),
            start: 0,
            initial_food: 0.,
            initial_mode: "cozy".into(),
            old_commits: 0,
            old_meals: 0,
            old_lines: 0,
            repos: vec![],
            events: vec![],
            seen: BTreeSet::new(),
            excludes: vec![],
        }
    }
    fn event(key: &str, at: u64, lines: u64, kind: &str) -> Event {
        Event {
            key: key.into(),
            at,
            added: lines,
            deleted: 0,
            files: 0,
            kind: kind.into(),
        }
    }
    #[test]
    fn missed_work_decays_and_modes_apply_at_event_time() {
        let mut p = pet();
        p.events.push(event("meal", 0, 25, "c"));
        assert_eq!(p.view(0).food, 20.);
        assert_eq!(p.view(72 * 3600).food, 0.);
        p.events.push(event("mode", 3600, 0, "traditional"));
        assert!((p.view(7200).food - (20. - 100. / 72. - 100. / 24.)).abs() < 0.00001);
    }
    #[test]
    fn bonuses_share_cap_and_legacy_commits_stay_line_only() {
        let mut p = pet();
        p.events.push(event("legacy", 0, 25, "c"));
        let mut commit = event("new", 1, 0, "n");
        commit.files = 2;
        p.events.push(commit);
        assert_eq!(p.view(1).earned, 32);
        p.events.push(event("manual", 2, 625, "m"));
        assert_eq!(p.view(2).earned, 100);
        let mut next = event("next", 86400, 0, "n");
        next.files = 1;
        p.events.push(next);
        assert_eq!(p.view(86400).earned, 107);
    }
    #[test]
    fn balanced_rewards_and_caps() {
        let mut p = pet();
        for (i, lines) in [5, 0, 0].into_iter().enumerate() {
            p.events.push(event(&i.to_string(), i as u64, lines, "b"));
        }
        assert_eq!(p.view(2).earned, 21);
        let mut p = pet();
        let mut e = event("first", 0, 650, "b");
        e.files = 3;
        p.events.push(e);
        p.events.push(event("second", 1, 0, "b"));
        assert_eq!(p.view(1).earned, 79);
        for i in 2..10 {
            let mut e = event(&i.to_string(), i, 0, "b");
            e.files = 1;
            p.events.push(e);
        }
        assert_eq!(p.view(10).earned, 100);
    }
    #[test]
    fn replay_is_order_independent_and_daily_budget_shared() {
        let mut p = pet();
        p.events = vec![event("b", 120, 75, "c"), event("a", 60, 25, "m")];
        let view = p.view(120);
        assert_eq!(view.earned, 40);
        p.events.reverse();
        assert_eq!(view.food, p.view(120).food);
        p.events.push(event("c", 180, 10000, "c"));
        assert_eq!(p.view(180).earned, 100);
        p.events.push(event("d", 86400, 25, "c"));
        assert_eq!(p.view(86400).earned, 120);
    }
}

#[cfg(test)]
mod review_regressions {
    use super::*;
    #[test]
    fn same_second_mode_changes_keep_insertion_order() {
        let mut p = Pet::new(
            "Test".into(),
            "cozy".into(),
            Binding {
                root: "/tmp".into(),
                common: "/tmp/.git".into(),
                baseline: String::new(),
                head: String::new(),
                since: 0,
            },
        );
        p.start = 0;
        // Lexical sorting put :10 before :9, silently restoring the wrong mode.
        p.events.push(Event {
            key: "manual:100:9".into(),
            at: 100,
            added: 0,
            deleted: 0,
            files: 0,
            kind: "traditional".into(),
        });
        p.events.push(Event {
            key: "manual:100:10".into(),
            at: 100,
            added: 0,
            deleted: 0,
            files: 0,
            kind: "cozy".into(),
        });
        assert_eq!(p.view(100).mode, "cozy");
        assert_eq!(p.view(101).mode, "cozy");
    }
}
