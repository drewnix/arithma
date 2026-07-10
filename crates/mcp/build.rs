//! Embed build provenance at compile time. A replay verdict is only
//! reproducible if the response names the checker build that produced it —
//! a hash written by the binary itself, never transcribed by hand.

use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn main() {
    let commit = git(&["rev-parse", "--short=12", "HEAD"]).unwrap_or_else(|| "unknown".into());
    // Dirty means the working tree does not match the named commit —
    // uncommitted changes or untracked files (conservative on purpose:
    // a binary that might not match its hash must say so).
    let dirty = git(&["status", "--porcelain"]).is_none_or(|s| !s.is_empty());

    println!("cargo:rustc-env=ARITHMA_GIT_COMMIT={commit}");
    println!("cargo:rustc-env=ARITHMA_GIT_DIRTY={dirty}");

    // Recompute when the checkout moves: HEAD changes on commit/checkout,
    // the index on staging. Without these, the embedded hash goes stale.
    if let Some(git_dir) = git(&["rev-parse", "--absolute-git-dir"]) {
        println!("cargo:rerun-if-changed={git_dir}/HEAD");
        println!("cargo:rerun-if-changed={git_dir}/index");
    }
}
