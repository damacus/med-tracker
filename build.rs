use std::process::Command;

fn git(arguments: &[&str]) -> Option<String> {
    let output = Command::new("git").args(arguments).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn main() {
    println!("cargo:rerun-if-env-changed=MEDTRACKER_VERSION");
    println!("cargo:rerun-if-env-changed=MEDTRACKER_COMMIT");
    let worktree = git(&["rev-parse", "--show-toplevel"]);
    if worktree.is_none() {
        println!("cargo:rerun-if-changed=build.rs");
    }
    let worktree = worktree.unwrap_or_else(|| "unknown".into());
    let mut commit = std::env::var("MEDTRACKER_COMMIT")
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| git(&["rev-parse", "--short", "HEAD"]))
        .unwrap_or_else(|| "unknown".into());
    if std::env::var("MEDTRACKER_COMMIT").is_err()
        && git(&["status", "--porcelain", "--untracked-files=no"])
            .is_some_and(|value| !value.is_empty())
    {
        commit.push_str(" (modified)");
    }
    let version = std::env::var("MEDTRACKER_VERSION")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_owned());
    println!("cargo:rustc-env=MEDTRACKER_BUILD_WORKTREE={worktree}");
    println!("cargo:rustc-env=MEDTRACKER_BUILD_COMMIT={commit}");
    println!("cargo:rustc-env=MEDTRACKER_BUILD_VERSION={version}+{commit}");
}
