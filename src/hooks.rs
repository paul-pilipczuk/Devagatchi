use crate::git;
use std::{
    env, fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
const MARKER: &str = "# Devagatchi managed hook v1";
const NAMES: [&str; 4] = ["post-commit", "post-merge", "post-rewrite", "pre-push"];
fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
fn script(name: &str) -> Result<String, String> {
    let exe = env::current_exe().map_err(|e| e.to_string())?;
    let exe = exe.to_str().ok_or("Executable path must be UTF-8")?;
    let action = match name {
        "post-commit" | "post-merge" => "commit",
        "post-rewrite" => "rewrite",
        "pre-push" => "push",
        _ => return Err("Unknown hook".into()),
    };
    Ok(format!("#!/bin/sh\n{MARKER}\n# Pet errors must never reject Git operations.\n{} hook {action} \"$@\" || :\n",quote(exe)))
}
fn directory(root: &str) -> Result<PathBuf, String> {
    // Resolve Git's effective hook location, including configured hooksPath.
    Ok(PathBuf::from(git::run(
        root,
        &["rev-parse", "--path-format=absolute", "--git-path", "hooks"],
    )?))
}
fn ordinary(path: &Path) -> Result<(), String> {
    if fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(format!("Refusing symlinked hook path: {}", path.display()));
    }
    Ok(())
}
pub fn install(push: bool) -> Result<(), String> {
    let root = git::root(".")?;
    let dir = directory(&root)?;
    ordinary(&dir)?;
    // Custom/shared hook managers may serve other repos. Supply manual integration instead.
    if git::run(&root, &["config", "--get", "core.hooksPath"]).is_ok() {
        return Err("Custom core.hooksPath detected; no hooks changed. Add `devagatchi hook commit` to your manager's post-commit/post-merge hook and `devagatchi hook rewrite \"$@\"` to post-rewrite. See help hooks.".into());
    }
    let names: Vec<_> = NAMES
        .into_iter()
        .filter(|s| push || *s != "pre-push")
        .collect();
    for name in &names {
        let path = dir.join(name);
        ordinary(&path)?;
        if path.exists() && fs::read_to_string(&path).map_err(|e| e.to_string())? != script(name)? {
            return Err(format!("Existing {name} hook preserved; install nothing. See help hooks for manual integration."));
        }
    }
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    for name in names {
        let path = dir.join(name);
        fs::write(&path, script(name)?).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
                .map_err(|e| e.to_string())?;
        }
    }
    println!(
        "Installed local commit, merge, and rewrite hooks{}.",
        if push { " and pre-push status" } else { "" }
    );
    Ok(())
}
pub fn uninstall() -> Result<(), String> {
    let root = git::root(".")?;
    if git::run(&root, &["config", "--get", "core.hooksPath"]).is_ok() {
        return Err("Custom core.hooksPath detected; shared hooks were left untouched. Remove the Devagatchi call through your hook manager.".into());
    }
    let dir = directory(&root)?;
    ordinary(&dir)?;
    for name in NAMES {
        let path = dir.join(name);
        ordinary(&path)?;
        if path.exists() {
            let contents = fs::read_to_string(&path).map_err(|e| e.to_string())?;
            if contents == script(name)? {
                fs::remove_file(path).map_err(|e| e.to_string())?;
                println!("Removed {name}");
            } else {
                println!("Kept {name}: not an unchanged Devagatchi hook");
            }
        }
    }
    Ok(())
}
pub fn status() -> Result<(), String> {
    let root = git::root(".")?;
    let dir = directory(&root)?;
    println!("Hooks directory: {}", dir.display());
    for name in NAMES {
        let path = dir.join(name);
        let contents = fs::read_to_string(&path).unwrap_or_default();
        println!(
            "{name}: {}",
            if contents == script(name)? {
                "installed"
            } else if path.exists() {
                "existing/custom"
            } else {
                "not installed"
            }
        );
    }
    Ok(())
}

// A short-lived supervisor gives Git its prompt back even during a large catch-up.
// No daemon survives this invocation. On Unix kill the worker's process group,
// including an in-flight Git statistics subprocess, when the time budget expires.
pub fn invoke(args: &[String]) -> Result<(), String> {
    if args.is_empty() || !matches!(args[0].as_str(), "commit" | "rewrite" | "push") {
        return Err("Usage: devagatchi hook commit|rewrite|push".into());
    }
    let mut command = Command::new(env::current_exe().map_err(|e| e.to_string())?);
    command
        .arg("__hook-worker")
        .args(args)
        .env("DEVAGATCHI_HOOK", "1")
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    let start = Instant::now();
    loop {
        if child.try_wait().map_err(|e| e.to_string())?.is_some() {
            return Ok(());
        }
        if start.elapsed() > Duration::from_millis(1000) {
            #[cfg(unix)]
            {
                let _ = Command::new("/bin/kill")
                    .arg("-TERM")
                    .arg(format!("-{}", child.id()))
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
            }
            let _ = child.kill();
            let _ = child.wait();
            eprintln!(
                "Devagatchi: work deferred to `devagatchi sync --catch-up` (hook time budget)."
            );
            return Ok(());
        }
        thread::sleep(Duration::from_millis(10));
    }
}
pub fn mappings() -> Result<Vec<(String, String)>, String> {
    let mut input = String::new();
    std::io::stdin()
        .take(1024 * 1024 + 1)
        .read_to_string(&mut input)
        .map_err(|e| e.to_string())?;
    if input.len() > 1024 * 1024 {
        return Err("Rewrite mapping too large; reconcile manually".into());
    }
    input
        .lines()
        .map(|line| {
            let f: Vec<_> = line.split_whitespace().collect();
            if f.len() < 2
                || f[0].is_empty()
                || !git::oid_or_empty(f[0])
                || !git::oid_or_empty(f[1])
            {
                return Err("Invalid rewrite mapping".into());
            }
            Ok((f[0].into(), f[1].into()))
        })
        .collect()
}
