//! Stamps the exact source revision into the binary.
//!
//! A running plane must be able to say which build it is (S27-T4). "Which Core
//! is running" is asked precisely when nobody can go and look, so it cannot be
//! answered by a file on disk beside the binary.

use std::path::Path;
use std::process::Command;

/// Repository paths the Rust workspace compiles from, including the files it
/// embeds with `include_str!`. "-dirty" describes these, not the whole tree.
const BUILD_INPUTS: &[&str] = &[
    ":/Cargo.toml",
    ":/Cargo.lock",
    ":/crates",
    ":/contracts",
    ":/tools",
    ":/docs/COMPANY_OPERATING_RULES.md",
    ":/docs/dogfood",
];

fn main() {
    watch_git_inputs();
    println!("cargo:rerun-if-env-changed=RESTLESS_SOURCE_REVISION");

    // An explicit value wins, so a release build from an exported tree (no
    // .git) still identifies itself.
    let revision = std::env::var("RESTLESS_SOURCE_REVISION")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(git_revision)
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=RESTLESS_SOURCE_REVISION={revision}");
}

fn git_output(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8(output.stdout).ok())
        .flatten()
}

fn watch_git_inputs() {
    // HEAD normally contains a symbolic reference and does not change when
    // another commit lands on that branch. Resolve Git paths instead of assuming
    // .git is a directory: linked worktrees keep their metadata elsewhere.
    // The index is deliberately not watched: staging changes no source, and
    // Git rewrites it on its own whenever it refreshes its stat cache.
    let mut paths = vec!["HEAD".to_string(), "packed-refs".to_string()];
    if let Some(reference) = git_output(&["symbolic-ref", "-q", "HEAD"]) {
        paths.push(reference.trim().to_owned());
    }
    for path in paths {
        if let Some(resolved) =
            git_output(&["rev-parse", "--path-format=absolute", "--git-path", &path])
        {
            let resolved = Path::new(resolved.trim());
            if resolved.exists() {
                println!("cargo:rerun-if-changed={}", resolved.display());
            } else if path.starts_with("refs/") {
                // A packed branch has no loose ref yet. Its parent catches the
                // next loose ref without making every build watch a missing file.
                if let Some(parent) = resolved.ancestors().skip(1).find(|path| path.is_dir()) {
                    println!("cargo:rerun-if-changed={}", parent.display());
                }
            }
        }
    }
    // A clean -> dirty transition must also refresh the embedded identity.
    // Watch only tracked build inputs: a web or docs edit cannot change this
    // binary and used to recompile the whole daemon.
    let mut ls_files = vec!["ls-files", "--full-name", "-z", "--"];
    ls_files.extend(BUILD_INPUTS);
    if let (Some(root), Some(files)) = (
        git_output(&["rev-parse", "--show-toplevel"]),
        git_output(&ls_files),
    ) {
        for file in files.split('\0').filter(|file| !file.is_empty()) {
            println!("cargo:rerun-if-changed={}/{}", root.trim(), file);
        }
    }
}

fn git_revision() -> Option<String> {
    let head = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())?;
    let head = String::from_utf8(head.stdout).ok()?.trim().to_string();
    if head.is_empty() {
        return None;
    }

    // A dirty tree is not the revision it claims to be. Say so rather than
    // reporting a commit that does not describe the running code. Without
    // optional locks, status does not rewrite the index while it looks.
    let dirty = Command::new("git")
        .args([
            "--no-optional-locks",
            "status",
            "--porcelain",
            "--untracked-files=no",
            "--",
        ])
        .args(BUILD_INPUTS)
        .output()
        .ok()
        .map(|output| !output.stdout.is_empty())
        .unwrap_or(false);

    Some(if dirty { format!("{head}-dirty") } else { head })
}
