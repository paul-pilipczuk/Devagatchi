#!/usr/bin/env python3
"""Local, dependency-free credential pattern check. Never prints matched values."""
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
PATTERNS = {
    "private key": rb"-----BEGIN (?:RSA |EC |OPENSSH |DSA )?PRIVATE KEY-----",
    "GitHub token": rb"\b(?:gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{50,})\b",
    "AWS access key": rb"\b(?:AKIA|ASIA)[A-Z0-9]{16}\b",
    "API token": rb"\bsk-(?:proj-|ant-)?[A-Za-z0-9_-]{32,}\b",
    "Slack token": rb"\bxox[baprs]-[A-Za-z0-9-]{20,}\b",
    "credential assignment": rb'''(?i)\b(?:api_key|api_secret|access_token|auth_token|password|client_secret)\s*[:=]\s*["'][^"'\s]{12,}["']''',
    "credential in URL": rb"https?://[^/\s:@]+:[^/\s@]+@",
}

def git(*args):
    return subprocess.check_output(["git", "-C", str(ROOT), *args])

def check(label, data):
    found = [name for name, pattern in PATTERNS.items() if re.search(pattern, data)]
    for name in found:
        print(f"Potential {name}: {label} (value redacted)")
    return bool(found)

def main():
    flagged = False
    files = set(git("ls-files", "--cached", "--others", "--exclude-standard", "-z").split(b"\0")) - {b""}
    for name in sorted(files):
        path = ROOT / name.decode()
        if path.is_file():
            flagged |= check(str(path.relative_to(ROOT)), path.read_bytes())
    objects = git("rev-list", "--objects", "--all").splitlines()
    scanned = 0
    for entry in objects:
        oid = entry.split(b" ", 1)[0].decode()
        kind = git("cat-file", "-t", oid).strip()
        if kind in (b"blob", b"commit", b"tag"):
            flagged |= check(f"Git object {oid[:12]}", git("cat-file", "-p", oid))
            scanned += 1
    print(f"Checked {len(files)} working-tree files and {scanned} history objects.")
    print("Review flagged candidates before publishing." if flagged else "No common secret patterns detected.")
    return int(flagged)

if __name__ == "__main__":
    sys.exit(main())
