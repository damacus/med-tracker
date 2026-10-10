use std::process::Command;

fn git(arguments: &[&str]) -> Option<String> {
    let output = Command::new("git").args(arguments).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn main() {
    let worktree = git(&["rev-parse", "--show-toplevel"]).unwrap_or_else(|| "unknown".into());
    let mut commit = git(&["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "unknown".into());
    if git(&["status", "--porcelain", "--untracked-files=no"])
        .is_some_and(|value| !value.is_empty())
    {
        commit.push_str(" (modified)");
    }
    println!("cargo:rustc-env=MEDTRACKER_BUILD_WORKTREE={worktree}");
    println!("cargo:rustc-env=MEDTRACKER_BUILD_COMMIT={commit}");
}
