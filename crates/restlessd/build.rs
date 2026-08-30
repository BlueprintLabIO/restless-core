//! Stamps the exact source revision into the binary.
//!
//! A running plane must be able to say which build it is (S27-T4). "Which Core
//! is running" is asked precisely when nobody can go and look, so it cannot be
//! answered by a file on disk beside the binary.

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../../.git/HEAD");
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
    // reporting a commit that does not describe the running code.
    let dirty = Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=no"])
        .output()
        .ok()
        .map(|output| !output.stdout.is_empty())
        .unwrap_or(false);

    Some(if dirty { format!("{head}-dirty") } else { head })
}
