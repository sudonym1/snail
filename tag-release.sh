#!/usr/bin/env bash
set -exuo pipefail

release_tag=${1?"usage: $0 vX.Y.Z"}
release_version=${release_tag#v}

# Idempotently skip creating an existing tag.
tag_exists=false
if git rev-parse -q --verify "refs/tags/${release_tag}" >/dev/null; then
  tag_exists=true
fi

update_package_version() {
  local file_path=$1
  sed -i "s/^version = \"[0-9.]*\"/version = \"${release_version}\"/" "${file_path}"
}

# Update project version and all crate versions.
update_package_version pyproject.toml
for cargo_file in crates/*/Cargo.toml extras/tree-sitter-snail/Cargo.toml; do
  update_package_version "${cargo_file}"
done

# Ensure CLI pulls the PyPI name for version metadata.
grep -q 'version("snail-lang")' python/snail/__init__.py

# Update the cargo lock file
make test

# Make a commit.

git add -A
if git diff --cached --quiet; then
  echo "No changes to commit." >&2
  exit 1
fi
git diff --cached
git commit -m "Tagging release $release_tag"

if [[ ${tag_exists} == false ]]; then
  git tag "${release_tag}"
fi

