use crate::{now, state::Binding};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

pub fn bytes(root: &str, args: &[&str]) -> Result<Vec<u8>, String> {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0");
    for key in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
        "GIT_COMMON_DIR",
        "GIT_PREFIX",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    ] {
        command.env_remove(key);
    }
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .spawn()
        .map_err(|e| format!("Cannot run Git: {e}"))?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let out = thread::spawn(move || {
        let mut b = Vec::new();
        stdout
            .take(16 * 1024 * 1024 + 1)
            .read_to_end(&mut b)
            .map(|_| b)
    });
    let err = thread::spawn(move || {
        let mut b = Vec::new();
        stderr.take(65536).read_to_end(&mut b).map(|_| b)
    });
    let started = Instant::now();
    let status = loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(s) => break s,
            None if started.elapsed() > Duration::from_secs(5) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(
                    "Git exceeded the 5-second limit; retry sync or use a smaller history batch"
                        .into(),
                );
            }
            None => thread::sleep(Duration::from_millis(5)),
        }
    };
    let stdout = out
        .join()
        .map_err(|_| "Git reader failed")?
        .map_err(|e| e.to_string())?;
    let stderr = err
        .join()
        .map_err(|_| "Git reader failed")?
        .map_err(|e| e.to_string())?;
    if stdout.len() > 16 * 1024 * 1024 {
        return Err("Git statistics exceeded the 16 MiB limit".into());
    }
    if !status.success() {
        return Err(format!(
            "Git failed: {}",
            String::from_utf8_lossy(&stderr).trim()
        ));
    }
    Ok(stdout)
}
pub fn run(root: &str, args: &[&str]) -> Result<String, String> {
    String::from_utf8(bytes(root, args)?)
        .map(|s| s.trim().into())
        .map_err(|_| "Invalid UTF-8 from Git".into())
}
pub fn root(path: &str) -> Result<String, String> {
    run(path, &["rev-parse", "--show-toplevel"])
}
pub fn common(root: &str) -> Result<String, String> {
    let path = run(
        root,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?;
    fs::canonicalize(path)
        .map_err(|e| e.to_string())?
        .to_str()
        .map(str::to_string)
        .ok_or("Git path must be UTF-8".into())
}
pub fn oid_or_empty(s: &str) -> bool {
    s.is_empty() || ((s.len() == 40 || s.len() == 64) && s.bytes().all(|b| b.is_ascii_hexdigit()))
}
pub fn head(root: &str) -> Result<String, String> {
    match run(root, &["rev-parse", "--verify", "HEAD^{commit}"]) {
        Ok(h) => Ok(h),
        Err(e) => {
            let branch = run(root, &["symbolic-ref", "-q", "HEAD"])?;
            if run(root, &["show-ref", "--verify", &branch]).is_err() {
                Ok(String::new())
            } else {
                Err(e)
            }
        }
    }
}
pub fn binding(path: &str) -> Result<Binding, String> {
    let root = root(path)?;
    let common = common(&root)?;
    let head = head(&root)?;
    Ok(Binding {
        root,
        common,
        baseline: head.clone(),
        head,
        since: now(),
    })
}
pub fn exclude_local(root: &str) -> Result<(), String> {
    let path = PathBuf::from(run(
        root,
        &[
            "rev-parse",
            "--path-format=absolute",
            "--git-path",
            "info/exclude",
        ],
    )?);
    if fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err("Refusing a symlinked Git exclude file".into());
    }
    let mut text = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e.to_string()),
    };
    if !text.lines().any(|l| l == "/.devagatchi.local*") {
        text.push_str("\n# Devagatchi private state\n/.devagatchi.local*\n");
        fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        fs::write(path, text).map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub fn excluded(path: &str, custom: &[String]) -> bool {
    let name = Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    path.split('/').any(|s| {
        matches!(
            s,
            "node_modules" | "vendor" | "target" | "dist" | "build" | ".git"
        )
    }) || name.starts_with(".devagatchi.local")
        || matches!(
            name,
            "Cargo.lock"
                | "package-lock.json"
                | "yarn.lock"
                | "pnpm-lock.yaml"
                | "poetry.lock"
                | "uv.lock"
                | "Gemfile.lock"
                | "go.sum"
        )
        || name.ends_with(".min.js")
        || name.ends_with(".min.css")
        || name.ends_with(".map")
        || custom
            .iter()
            .any(|p| path == p || path.starts_with(&format!("{}/", p.trim_end_matches('/'))))
}
pub fn stats(root: &str, id: &str, custom: &[String]) -> Result<(u64, u64), String> {
    if !oid_or_empty(id) || id.is_empty() {
        return Err("Invalid commit ID".into());
    }
    // Merges have no additional meal; their ordinary ancestor commits are processed separately.
    if run(root, &["rev-list", "--parents", "-n", "1", id])?
        .split_whitespace()
        .count()
        > 2
    {
        return Ok((0, 0));
    }
    let raw = bytes(
        root,
        &[
            "diff-tree",
            "--root",
            "--no-commit-id",
            "-r",
            "-M",
            "-l1000",
            "--ignore-all-space",
            "--no-ext-diff",
            "--no-textconv",
            "--numstat",
            "-z",
            id,
        ],
    )?;
    parse_stats(&raw, custom)
}
fn parse_stats(raw: &[u8], custom: &[String]) -> Result<(u64, u64), String> {
    let mut fields = raw.split(|b| *b == 0);
    let (mut a, mut d) = (0u64, 0u64);
    while let Some(row) = fields.next() {
        if row.is_empty() {
            continue;
        }
        let mut parts = row.splitn(3, |b| *b == b'\t');
        let add = parts.next().ok_or("Missing additions")?;
        let del = parts.next().ok_or("Missing deletions")?;
        let path = parts.next().ok_or("Missing path")?;
        let mut paths = vec![path];
        if path.is_empty() {
            paths = vec![
                fields.next().ok_or("Missing old path")?,
                fields.next().ok_or("Missing new path")?,
            ];
        }
        if add == b"-"
            || del == b"-"
            || paths
                .iter()
                .any(|p| excluded(&String::from_utf8_lossy(p), custom))
        {
            continue;
        }
        let number = |b: &[u8]| -> Result<u64, String> {
            std::str::from_utf8(b)
                .map_err(|_| "Invalid diff count")?
                .parse()
                .map_err(|_| "Invalid diff count".into())
        };
        a = a.checked_add(number(add)?).ok_or("Too many additions")?;
        d = d.checked_add(number(del)?).ok_or("Too many deletions")?;
    }
    Ok((a, d))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stats_are_nul_safe() {
        assert_eq!(
            parse_stats(b"2\t3\tfile\nwith\ttabs\0-\t-\tbinary\0", &[]).unwrap(),
            (2, 3)
        );
        assert_eq!(parse_stats(b"1\t0\t\0old\0new\0", &[]).unwrap(), (1, 0));
        assert_eq!(parse_stats(b"100\t0\tCargo.lock\0", &[]).unwrap(), (0, 0));
    }
}
