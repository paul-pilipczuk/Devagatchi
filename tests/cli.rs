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
    fn command(&self, args: &[&str]) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_devagatchi"));
        c.args(args)
            .current_dir(self.0.join("repo"))
            .env("XDG_DATA_HOME", self.0.join("data"))
            .env("NO_COLOR", "1");
        c
    }
    fn cli(&self, args: &[&str]) -> Output {
        self.command(args).output().unwrap()
    }
    fn ok(&self, args: &[&str]) -> String {
        let o = self.cli(args);
        assert!(
            o.status.success(),
            "{:?}: {}",
            args,
            String::from_utf8_lossy(&o.stderr)
        );
        String::from_utf8(o.stdout).unwrap()
    }
    fn git(&self, args: &[&str]) -> String {
        let o = Command::new("git")
            .arg("-C")
            .arg(self.0.join("repo"))
            .args(args)
            .env("XDG_DATA_HOME", self.0.join("data"))
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
            .output()
            .unwrap();
        assert!(
            o.status.success(),
            "Git: {}",
            String::from_utf8_lossy(&o.stderr)
        );
        String::from_utf8_lossy(&o.stdout).into()
    }
    fn setup(&self) {
        self.git(&["init", "-q"]);
        self.git(&["config", "commit.gpgsign", "false"]);
    }
    fn commit(&self, name: &str, contents: &str) {
        fs::write(self.0.join("repo").join(name), contents).unwrap();
        self.git(&["add", name]);
        self.git(&["commit", "-qm", "work"]);
    }
    fn global(&self) -> PathBuf {
        self.0.join("data/devagatchi/pet.txt")
    }
}
impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn scopes_keep_pets_independent_and_ignore_local_data() {
    let s = Sandbox::new();
    s.setup();
    s.ok(&["init", "Mochi"]);
    s.ok(&["init", "Scout", "--repo"]);
    assert!(s.ok(&["status"]).contains("Mochi"));
    assert!(s.ok(&["status"]).contains("Scout"));
    assert!(!s.cli(&["feed", "25"]).status.success());
    s.ok(&["feed", "25", "--both"]);
    assert!(s.ok(&["status", "--global"]).contains("Food earned: 20"));
    assert!(s.ok(&["status", "--repo"]).contains("Food earned: 20"));
    assert!(s
        .git(&["check-ignore", ".devagatchi.local"])
        .contains(".devagatchi.local"));
    let before = fs::read(s.global()).unwrap();
    assert!(!s.cli(&["init", "Replace"]).status.success());
    assert_eq!(before, fs::read(s.global()).unwrap());
    assert!(!s.cli(&["init", "X", "--repo", "--global"]).status.success());
}
#[test]
fn line_stats_fan_out_and_sync_is_idempotent() {
    let s = Sandbox::new();
    s.setup();
    s.ok(&["init", "Mochi"]);
    s.ok(&["init", "Scout", "--repo"]);
    s.commit("file.txt", &"hello\n".repeat(25));
    let out = s.ok(&["sync"]);
    assert_eq!(out.matches("+25 / -0 lines, +20 food").count(), 2);
    let state = fs::read(s.global()).unwrap();
    assert!(s.ok(&["sync"]).contains("0 new commit(s)"));
    assert_eq!(state, fs::read(s.global()).unwrap());
    s.commit("file.txt", &"hello\n".repeat(100));
    let out = s.ok(&["sync"]);
    assert_eq!(out.matches("+75 / -0 lines, +20 food").count(), 2);
    s.commit("Cargo.lock", &"generated\n".repeat(100));
    assert!(s.ok(&["sync"]).contains("+0 / -0 lines, +0 food"));
    s.git(&["mv", "file.txt", "renamed.txt"]);
    s.git(&["commit", "-qm", "rename"]);
    assert!(s.ok(&["sync"]).contains("+0 / -0 lines, +0 food"));
    s.commit("renamed.txt", &"hello\n".repeat(90));
    assert!(s.ok(&["sync"]).contains("+0 / -10 lines"));
}
#[test]
fn legacy_migration_preserves_identity_and_totals() {
    let s = Sandbox::new();
    s.setup();
    s.commit("first", "a\n");
    let head = s.git(&["rev-parse", "HEAD"]);
    let root = s.0.join("repo").to_string_lossy().to_string();
    let hex: String = root.bytes().map(|b| format!("{b:02x}")).collect();
    fs::create_dir_all(s.global().parent().unwrap()).unwrap();
    fs::write(
        s.global(),
        format!(
            "devagatchi-v1\n4d6f636869\ncozy\n{hex}\n{}\n8\n42\n1\ndog\n3\ngreen\n",
            head.trim()
        ),
    )
    .unwrap();
    let out = s.ok(&["status"]);
    assert!(out.contains("Mochi the dog"));
    assert!(out.contains("Commits spotted: 8"));
    assert!(out.contains("Lines fed (lifetime): 42"));
    s.ok(&["color", "cyan"]);
    assert!(fs::read_to_string(s.global())
        .unwrap()
        .starts_with("devagatchi-v2"));
    assert!(s.ok(&["sync"]).contains("0 new commit(s)"));
    assert!(s.global().with_extension("txt.v1.bak").exists());
}
#[test]
fn manual_and_git_lines_share_budget_and_invalid_inputs_do_not_write() {
    let s = Sandbox::new();
    s.setup();
    s.ok(&["init", "M"]);
    s.ok(&["feed", "25"]);
    s.commit("x", &"x\n".repeat(75));
    assert!(s.ok(&["sync"]).contains("+20 food"));
    let before = fs::read(s.global()).unwrap();
    for args in [
        vec!["feed", "0"],
        vec!["feed", "-1"],
        vec!["color", "invalid"],
        vec!["mode", "invalid"],
        vec!["exclude", "../outside"],
    ] {
        assert!(!s.cli(&args).status.success());
    }
    assert_eq!(before, fs::read(s.global()).unwrap());
}
#[test]
fn global_multiple_repositories_and_custom_excludes() {
    let s = Sandbox::new();
    s.setup();
    s.ok(&["init", "M"]);
    s.ok(&["exclude", "generated"]);
    fs::create_dir_all(s.0.join("repo/generated")).unwrap();
    s.commit("generated/output", "x\n");
    assert!(s.ok(&["sync"]).contains("+0 food"));
    let other = s.0.join("other");
    fs::create_dir(&other).unwrap();
    assert!(Command::new("git")
        .args(["init", "-q"])
        .arg(&other)
        .status()
        .unwrap()
        .success());
    let out = s
        .command(&["connect"])
        .current_dir(&other)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(s
        .ok(&["status", "--global"])
        .contains("Connected repositories: 2"));
}
#[test]
fn help_before_setup_and_all_settings() {
    let s = Sandbox::new();
    assert!(s.ok(&[]).contains("USAGE"));
    for c in [
        "init", "status", "feed", "mode", "pet", "color", "connect", "exclude", "help",
    ] {
        assert!(s.ok(&[c, "--help"]).contains("USAGE"));
    }
    s.setup();
    s.ok(&["init", "M"]);
    for c in [
        "cyan", "green", "yellow", "blue", "magenta", "red", "white", "off",
    ] {
        assert!(!s.ok(&["color", c]).contains('\x1b'));
    }
    for pet in ["cat", "dog", "bird", "lizard"] {
        assert!(s.ok(&["pet", pet]).contains(&format!("M the {pet}")));
    }
}

#[test]
fn hooks_feed_both_preserve_amends_and_uninstall_cleanly() {
    let s = Sandbox::new();
    s.setup();
    s.ok(&["init", "M"]);
    s.ok(&["init", "R", "--repo"]);
    s.ok(&["hooks", "install", "--push"]);
    assert!(s
        .ok(&["hooks", "status"])
        .contains("post-commit: installed"));
    s.commit("food.txt", &"x\n".repeat(25));
    assert!(s.ok(&["status", "--global"]).contains("Food earned: 20"));
    assert!(s.ok(&["status", "--repo"]).contains("Food earned: 20"));
    s.git(&["commit", "--amend", "--no-edit"]);
    assert!(s.ok(&["sync"]).contains("0 new commit(s)"));
    assert!(s.ok(&["status", "--global"]).contains("Commits spotted: 1"));
    let before = fs::read(s.global()).unwrap();
    s.ok(&["hook", "push"]);
    assert_eq!(before, fs::read(s.global()).unwrap());
    s.ok(&["hooks", "uninstall"]);
    assert!(s
        .ok(&["hooks", "status"])
        .contains("post-commit: not installed"));
}
#[test]
fn existing_hooks_and_shared_managers_are_untouched() {
    let s = Sandbox::new();
    s.setup();
    s.ok(&["init", "M"]);
    let path = s.0.join("repo/.git/hooks/post-commit");
    fs::write(&path, "#!/bin/sh\necho existing\n").unwrap();
    assert!(!s.cli(&["hooks", "install"]).status.success());
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "#!/bin/sh\necho existing\n"
    );
    s.ok(&["hooks", "uninstall"]);
    assert!(path.exists());
    s.git(&["config", "core.hooksPath", "custom-hooks"]);
    assert!(!s.cli(&["hooks", "install"]).status.success());
    assert!(!s.0.join("repo/custom-hooks").exists());
}
#[test]
fn catch_up_is_bounded_and_does_not_skip_branches() {
    let s = Sandbox::new();
    s.setup();
    s.ok(&["init", "M"]);
    // Empty commits still exercise the full traversal and duplicate ledger.
    for _ in 0..201 {
        s.git(&["commit", "--allow-empty", "-qm", "work"]);
    }
    assert!(!s.cli(&["sync"]).status.success());
    assert!(s
        .ok(&["sync", "--catch-up"])
        .contains("More history remains"));
    assert!(s.ok(&["sync", "--catch-up"]).contains("1 new commit(s)"));
    assert!(s.ok(&["status"]).contains("Commits spotted: 201"));
    assert!(s.ok(&["sync"]).contains("0 new commit(s)"));
}
#[test]
fn worktrees_share_global_duplicate_protection_but_keep_local_pets() {
    let s = Sandbox::new();
    s.setup();
    s.commit("baseline", "b\n");
    s.ok(&["init", "G"]);
    let other = s.0.join("worktree");
    s.git(&["worktree", "add", "-b", "other", other.to_str().unwrap()]);
    assert!(s
        .command(&["connect"])
        .current_dir(&other)
        .output()
        .unwrap()
        .status
        .success());
    assert!(s
        .command(&["init", "R", "--repo"])
        .current_dir(&other)
        .output()
        .unwrap()
        .status
        .success());
    s.commit("food", "x\n");
    s.ok(&["sync"]);
    let head = s.git(&["rev-parse", "HEAD"]);
    let out = Command::new("git")
        .arg("-C")
        .arg(&other)
        .args(["merge", "--ff-only", head.trim()])
        .output()
        .unwrap();
    assert!(out.status.success());
    let out = s.command(&["sync"]).current_dir(&other).output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(s.ok(&["status", "--global"]).contains("Commits spotted: 1"));
}
#[test]
fn concurrent_feeding_preserves_state_and_locks_release_after_exit() {
    let s = Sandbox::new();
    s.setup();
    s.ok(&["init", "M"]);
    let mut children = vec![];
    for _ in 0..8 {
        children.push(
            s.command(&["feed", "1"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .unwrap(),
        );
    }
    let successes = children
        .iter_mut()
        .map(|c| c.wait().unwrap().success())
        .filter(|success| *success)
        .count();
    assert!(s
        .ok(&["status"])
        .contains(&format!("Lines fed (lifetime): {successes}")));
    s.ok(&["feed", "1"]);
}

#[cfg(unix)]
#[test]
fn hook_timeout_terminates_worker_and_git_subprocesses() {
    use std::os::unix::fs::PermissionsExt;
    let s = Sandbox::new();
    s.setup();
    s.ok(&["init", "M"]);
    let bin = s.0.join("bin");
    fs::create_dir(&bin).unwrap();
    let shim = bin.join("git");
    fs::write(&shim, "#!/bin/sh\nsleep 10\n").unwrap();
    fs::set_permissions(&shim, fs::Permissions::from_mode(0o755)).unwrap();
    let start = std::time::Instant::now();
    let out = s
        .command(&["hook", "commit"])
        .env(
            "PATH",
            format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()),
        )
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("hook time budget"));
    assert!(start.elapsed() < std::time::Duration::from_secs(3));
    // Lock acquisition still works after timeout.
    s.ok(&["feed", "1"]);
}
