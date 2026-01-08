---
name: release
description: Prepare and create a new release - runs all CI checks, updates version in ALL Cargo.toml files in the repo, creates git tag, and pushes to remote. Use when the user wants to cut a new release or tag a version.
---

# Release Preparation Tool

This tool automates the release preparation workflow for the Snail project.

## When to Use

Invoke this tool when the user wants to:
- Create a new release
- Tag a version
- Prepare for publishing
- Cut a release

## What This Skill Does

1. **Validates the version** - Runs `update_version.sh` script to ensure:
   - Version format matches vX.Y.Z (e.g., v1.2.3)
   - No leading zeros in version components
   - Git tag doesn't already exist
   - All Cargo.toml files in the repo have consistent versions
2. **Runs all mandatory CI checks** - Executes the same checks that CI runs:
   - `cargo fmt --check` - Ensures code is properly formatted
   - `RUSTFLAGS="-D warnings" cargo build` - Builds with zero warnings
   - `cargo clippy -- -D warnings` - Lints with zero warnings
   - `RUSTFLAGS="-D warnings" cargo test` - Runs all tests
3. **Creates and commits version bump** - Commits the Cargo.toml and
   Cargo.lock changes
4. **Creates and pushes git tag** - Tags the release and pushes to remote

## Required Input

The user must provide a version tag in the format `vX.Y.Z` where X, Y, and Z are integers.

Examples of valid versions: `v1.0.0`, `v2.3.1`, `v0.1.0`

## Instructions

When this skill is invoked:

1. **Get the version tag**:
   - If the user provided a version argument, use it
   - Otherwise, ask the user for the version tag
   - The version must be in format `vX.Y.Z` (e.g., v1.2.3)

2. **MANDATORY: Run the validation and Cargo.toml update script**:
   **CRITICAL**: You MUST run this script as the very first validation step, before proceeding with any other operations.
   ```bash
   .gemini/tools/release/update_version.sh <version>
   ```
   - Replace `<version>` with the version tag from step 1
   - This script performs comprehensive validation:
     - Checks version format (vX.Y.Z)
     - Validates all Cargo.toml files have consistent versions
   - If the script fails (non-zero exit code), STOP immediately and report the error to the user
   - Do NOT proceed to CI checks or any other steps if validation fails
   - The script provides detailed error messages explaining what went wrong

4. **Run all CI checks** (in this exact order):
   ```bash
   cargo fmt --check
   RUSTFLAGS="-D warnings" cargo build
   cargo clippy -- -D warnings
   RUSTFLAGS="-D warnings" cargo test
   ```
   - If ANY check fails, STOP immediately and report the failure to the user
   - Do NOT proceed to version updates or tagging if CI fails
   - The user must fix the issues before creating a release

5. **Create version bump commit**:
   ```bash
   git add -A
   git commit -m "Bump version to <version>"
   ```
   - Replace `<version>` with the version number (with 'v' prefix, e.g., "Bump version to v1.2.3")

6. **Create and push the git tag**:
   ```bash
   git tag <version>
   git push origin <version>
   ```
   - Replace `<version>` with the full version tag (e.g., `v1.2.3`)
   - Push both the tag and the version bump commit

7. **Report completion**:
   - Confirm to the user that the release has been prepared
   - Show the version tag that was created
   - Mention that the tag and version bump have been pushed to remote
   - Optionally remind them about any manual release steps (e.g., creating GitHub release, publishing to crates.io)

## Error Handling

- If validation script fails: Stop immediately and show the error message from the script. The script provides detailed explanations of what validation failed.
- If CI checks fail: Stop and report which check failed. Do not proceed.
- If git operations fail: Report the error and suggest checking git status.
- If Cargo.toml files are not found or cannot be updated: Report the issue and ask for guidance.

## Example Invocation

User: `/release v1.2.3`

or

User: `/release`
Assistant: What version would you like to release? (format: vX.Y.Z)

## Notes

- This tool follows the project's mandatory CI requirements from AGENTS.md
- The `update_version.sh` script is located in `.gemini/tools/release/` and MUST be invoked as the first validation step
- All validation and CI checks must pass and the version updates must be
  commited before tagging
- Version tags use semantic versioning with 'v' prefix (e.g., v1.2.3)
- The skill both creates the tag locally and pushes it to the remote repository
- The validation script ensures all Cargo.toml files have the same version before proceeding
