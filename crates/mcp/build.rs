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

    // Emitting ANY rerun-if-changed disables cargo's default heuristics,
    // so the script must also watch the sources whose edits make the tree
    // dirty — otherwise the baked dirty flag reports the state of the LAST
    // script run, and a probe can read "dirty: false" from a dirty tree.
    // A provenance sentinel that can go stale defeats its purpose; the
    // walk costs milliseconds.
    println!("cargo:rerun-if-changed=build.rs");
    for root in ["src", "../../src", "../cli/src"] {
        emit_rerun_for_rs_files(std::path::Path::new(root));
    }
}

fn emit_rerun_for_rs_files(dir: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            emit_rerun_for_rs_files(&path);
        } else if path.extension().is_some_and(|e| e == "rs") {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
}
