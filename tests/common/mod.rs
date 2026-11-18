#![allow(dead_code)]
use std::path::Path;
use std::process::Command;

pub fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .expect("git");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Repo temporal con un commit: src/a.rs (3 líneas) y src/b.py (2 líneas).
pub fn repo() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    git(d.path(), &["init", "-q"]);
    std::fs::create_dir_all(d.path().join("src")).unwrap();
    std::fs::write(d.path().join("src/a.rs"), "a\nb\nc\n").unwrap();
    std::fs::write(d.path().join("src/b.py"), "x\ny\n").unwrap();
    std::fs::write(d.path().join(".gitignore"), ".tara/\n").unwrap();
    git(d.path(), &["add", "."]);
    git(d.path(), &["commit", "-q", "-m", "uno"]);
    d
}

pub fn tara() -> assert_cmd::Command {
    let mut c = assert_cmd::Command::cargo_bin("tara").unwrap();
    c.env("TARA_NOW", "2026-01-01T00:00:00Z");
    c
}
