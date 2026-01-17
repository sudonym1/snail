use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn git_output(args: &[&str], cwd: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Some(stdout.trim().to_string())
}

fn expected_tags(version: &str) -> HashSet<String> {
    let mut tags = HashSet::new();
    if version.is_empty() {
        return tags;
    }
    if let Some(stripped) = version.strip_prefix('v') {
        tags.insert(version.to_string());
        tags.insert(stripped.to_string());
    } else {
        tags.insert(version.to_string());
        tags.insert(format!("v{version}"));
    }
    tags
}

fn rerun_if_changed(path: PathBuf) {
    println!("cargo:rerun-if-changed={}", path.display());
}

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));

    if let Some(rev) = git_output(&["rev-parse", "--short", "HEAD"], &manifest_dir)
        && !rev.is_empty()
    {
        println!("cargo:rustc-env=SNAIL_GIT_SHA={rev}");

        if let Some(status) = git_output(&["status", "--porcelain"], &manifest_dir) {
            let dirty = !status.is_empty();
            println!(
                "cargo:rustc-env=SNAIL_GIT_DIRTY={}",
                if dirty { "true" } else { "false" }
            );
        }

        if let Some(tags_output) = git_output(&["tag", "--points-at", "HEAD"], &manifest_dir) {
            let version = env::var("CARGO_PKG_VERSION").unwrap_or_default();
            let expected = expected_tags(&version);
            let tags: HashSet<&str> = tags_output
                .lines()
                .map(str::trim)
                .filter(|tag| !tag.is_empty())
                .collect();
            let tagged = tags.iter().any(|tag| expected.contains(*tag));
            let untagged = !tagged;
            println!(
                "cargo:rustc-env=SNAIL_GIT_UNTAGGED={}",
                if untagged { "true" } else { "false" }
            );
        }
    }

    if let Some(git_dir) = git_output(&["rev-parse", "--git-dir"], &manifest_dir) {
        let git_dir = PathBuf::from(git_dir);
        let git_dir = if git_dir.is_relative() {
            manifest_dir.join(git_dir)
        } else {
            git_dir
        };
        rerun_if_changed(git_dir.join("HEAD"));
        rerun_if_changed(git_dir.join("index"));
        rerun_if_changed(git_dir.join("packed-refs"));
    }
}
