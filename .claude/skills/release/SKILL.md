---
name: release
description: Prepare and create a new release - runs all CI checks, updates version in ALL Cargo.toml files in the repo, creates git tag, and pushes to remote. Use when the user wants to cut a new release or tag a version.
---

# Release Preparation Skill

This skill automates the release preparation workflow for the Snail project.

## When to Use

Invoke this skill when the user wants to:
- Create a new release
- Tag a version
- Prepare for publishing
- Cut a release

## What This Skill Does

1. **Validates the version tag format** - Ensures the version follows vX.Y.Z format (e.g., v1.2.3)
2. **Runs all mandatory CI checks** - Executes the same checks that CI runs:
   - `cargo fmt --check` - Ensures code is properly formatted
   - `RUSTFLAGS="-D warnings" cargo build` - Builds with zero warnings
   - `cargo clippy -- -D warnings` - Lints with zero warnings
   - `RUSTFLAGS="-D warnings" cargo test` - Runs all tests
3. **Updates Cargo.toml version** - Updates the version field in all relevant Cargo.toml files to match the release version (without the 'v' prefix)
4. **Creates and commits version bump** - Commits the Cargo.toml changes
5. **Creates and pushes git tag** - Tags the release and pushes to remote

## Required Input

The user must provide a version tag in the format `vX.Y.Z` where X, Y, and Z are integers.

Examples of valid versions: `v1.0.0`, `v2.3.1`, `v0.1.0`

## Instructions

When this skill is invoked:

1. **Extract and validate the version tag**:
   - If the user provided a version argument, use it
   - Otherwise, ask the user for the version tag
   - Validate the format matches `vX.Y.Z` (regex: `^v\d+\.\d+\.\d+$`)
   - If invalid, explain the correct format and ask again

2. **Run all CI checks** (in this exact order):
   ```bash
   cargo fmt --check
   RUSTFLAGS="-D warnings" cargo build
   cargo clippy -- -D warnings
   RUSTFLAGS="-D warnings" cargo test
   ```
   - If ANY check fails, STOP immediately and report the failure to the user
   - Do NOT proceed to version updates or tagging if CI fails
   - The user must fix the issues before creating a release

3. **Update version in Cargo.toml files**:
   - Extract the version number without the 'v' prefix (e.g., `v1.2.3` → `1.2.3`)
   - Update the `version` field in the workspace root `Cargo.toml`
   - Update the `version` field in all workspace member crates' `Cargo.toml` files
   - Use the Read tool to find all Cargo.toml files that need updating
   - Use the Edit tool to update each version field

4. **Create version bump commit**:
   ```bash
   git add Cargo.toml crates/*/Cargo.toml
   git commit -m "Bump version to <version>"
   ```
   - Replace `<version>` with the version number (with 'v' prefix, e.g., "Bump version to v1.2.3")

5. **Create and push the git tag**:
   ```bash
   git tag <version>
   git push origin <version>
   git push origin HEAD
   ```
   - Replace `<version>` with the full version tag (e.g., `v1.2.3`)
   - Push both the tag and the version bump commit

6. **Report completion**:
   - Confirm to the user that the release has been prepared
   - Show the version tag that was created
   - Mention that the tag and version bump have been pushed to remote
   - Optionally remind them about any manual release steps (e.g., creating GitHub release, publishing to crates.io)

## Error Handling

- If CI checks fail: Stop and report which check failed. Do not proceed.
- If version format is invalid: Explain the format and ask for a corrected version.
- If git operations fail: Report the error and suggest checking git status.
- If Cargo.toml files are not found or cannot be updated: Report the issue and ask for guidance.

## Example Invocation

User: `/release v1.2.3`

or

User: `/release`
Assistant: What version would you like to release? (format: vX.Y.Z)

## Notes

- This skill follows the project's mandatory CI requirements from CLAUDE.md
- All CI checks must pass before version updates and tagging
- Version tags use semantic versioning with 'v' prefix
- The skill both creates the tag locally and pushes it to the remote repository
