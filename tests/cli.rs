use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Sandbox(PathBuf);
impl Sandbox {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "devagatchi-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("repo")).unwrap();
        Self(root)
    }
    fn cli(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_devagatchi"))
            .args(args)
            .current_dir(self.0.join("repo"))
            .env("XDG_DATA_HOME", self.0.join("data"))
            .output()
            .unwrap()
    }
    fn ok(&self, args: &[&str]) -> String {
        let out = self.cli(args);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap()
    }
    fn git(&self, args: &[&str]) {
        let out = Command::new("git")
            .arg("-C")
            .arg(self.0.join("repo"))
            .args(args)
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}
impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn help_works_before_setup_and_rejects_unknown_commands() {
    let s = Sandbox::new();
    assert!(s.ok(&[]).contains("COMMANDS"));
    for command in [
        "init", "status", "sync", "feed", "pet", "mode", "color", "help",
    ] {
        assert!(s.ok(&[command, "--help"]).contains("USAGE"));
        assert!(s.ok(&["help", command]).contains("USAGE"));
    }
    assert!(s.ok(&["--version"]).contains("devagatchi"));
    assert!(!s.cli(&["help", "unknown"]).status.success());
    assert!(!s.cli(&["status", "extra"]).status.success());
}
#[test]
fn sync_feeds_once_and_existing_pets_survive_changes() {
    let s = Sandbox::new();
    s.git(&["init", "-q"]);
    s.ok(&["init", "Mochi"]);
    s.ok(&["feed", "42"]);
    let path = s.0.join("data/devagatchi/pet.txt");
    let mut old: Vec<String> = fs::read_to_string(&path)
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect();
    old[7] = "1".into();
    fs::write(&path, old.join("\n") + "\n").unwrap();
    s.git(&[
        "-c",
        "core.hooksPath=/dev/null",
        "-c",
        "commit.gpgsign=false",
        "commit",
        "--allow-empty",
        "-qm",
        "test",
    ]);
    let out = s.ok(&["sync"]);
    assert!(out.contains("1 new commit(s)"));
    assert!(out.contains("ate 1 commit meal(s)"));
    assert!(out.contains("Commit meals (lifetime): 1"));
    assert!(out.contains("Lines fed (lifetime): 42"));
    let after = fs::read_to_string(&path).unwrap();
    assert_ne!(after.lines().nth(7), Some("1"));
    assert!(s.ok(&["sync"]).contains("No new commits"));
    assert_eq!(after, fs::read_to_string(&path).unwrap());
    // Moving backwards must not grant a meal or change progress.
    s.git(&[
        "-c",
        "core.hooksPath=/dev/null",
        "-c",
        "commit.gpgsign=false",
        "commit",
        "--allow-empty",
        "-qm",
        "second",
    ]);
    assert!(s.ok(&["sync"]).contains("Commit meals (lifetime): 2"));
    let latest = fs::read(&path).unwrap();
    s.git(&["checkout", "--detach", "HEAD~1"]);
    assert!(!s.cli(&["sync"]).status.success());
    assert_eq!(latest, fs::read(&path).unwrap());
    // Original eight-line saves had no species field.
    fs::write(
        &path,
        after.lines().take(8).collect::<Vec<_>>().join("\n") + "\n",
    )
    .unwrap();
    assert!(s.ok(&[]).contains("Mochi the cat"));
    for species in ["cat", "dog", "bird", "lizard"] {
        let out = s.ok(&["pet", species]);
        assert!(out.contains(&format!("Mochi the {species}")));
        assert!(out.contains("Lines fed (lifetime): 42"));
    }
    for color in [
        "cyan", "green", "yellow", "blue", "magenta", "red", "white", "off",
    ] {
        let out = s.ok(&["color", color]);
        assert!(!out.contains('\x1b')); // Captured output must stay plain.
        assert_eq!(
            fs::read_to_string(&path).unwrap().lines().nth(10),
            Some(color)
        );
    }
    let before = fs::read(&path).unwrap();
    for args in [
        vec!["feed", "0"],
        vec!["feed", "-1"],
        vec!["pet", "snake"],
        vec!["color", "invalid"],
        vec!["init", "Other"],
    ] {
        assert!(!s.cli(&args).status.success());
    }
    assert_eq!(before, fs::read(&path).unwrap());
    let out = s.ok(&["feed", "8"]);
    assert!(out.contains("ate 8 lines"));
    assert!(out.contains("Lines fed (lifetime): 50"));
}

#[test]
fn status_explains_care_and_mode_changes_preserve_timer() {
    let s = Sandbox::new();
    s.git(&["init", "-q"]);
    let out = s.ok(&["init", "Mochi"]);
    assert!(out.contains("Mood: happy | Care mode: cozy"));
    assert!(out.contains("Next: content in"));
    let path = s.0.join("data/devagatchi/pet.txt");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    for (age, cozy, traditional, eye) in [
        (2, "content", "content", "o"),
        (25, "content", "hungry", "O"),
        (49, "content", "very hungry", ";"),
        (73, "sleepy", "very hungry", ";"),
    ] {
        let mut state: Vec<String> = fs::read_to_string(&path)
            .unwrap()
            .lines()
            .map(str::to_string)
            .collect();
        state[2] = "cozy".into();
        state[7] = (now - age * 3600).to_string();
        let timestamp = state[7].clone();
        fs::write(&path, state.join("\n") + "\n").unwrap();
        let before = fs::read(&path).unwrap();
        assert!(s
            .ok(&["status"])
            .contains(&format!("Mood: {cozy} | Care mode: cozy")));
        assert_eq!(before, fs::read(&path).unwrap());
        let out = s.ok(&["mode", "traditional"]);
        assert!(out.contains(&format!("Mood: {traditional} | Care mode: traditional")));
        assert!(out.contains(&format!("({eye})")));
        assert_eq!(
            fs::read_to_string(&path).unwrap().lines().nth(7),
            Some(timestamp.as_str())
        );
        assert!(out.contains("Care age:"));
        assert!(out.contains("Next:"));
    }
    let out = s.ok(&["feed", "1"]);
    assert!(out.contains("Mood: happy"));
    assert!(out.contains("(^)"));
    assert!(out.contains("Next: content in"));
    for args in [
        vec!["--help"],
        vec!["status", "--help"],
        vec!["init", "--help"],
        vec!["mode", "--help"],
    ] {
        let out = s.ok(&args);
        assert!(out.contains("happy for the first hour"));
        assert!(out.contains("72 hours"));
        assert!(out.contains("24 hours"));
        assert!(out.contains("48 hours"));
    }
}
