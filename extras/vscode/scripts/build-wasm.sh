#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/../../.." && pwd)"
ts_dir="${repo_root}/extras/tree-sitter-snail"
vs_dir="${repo_root}/extras/vscode"
query_src="${ts_dir}/queries/highlights.scm"
query_dest="${vs_dir}/queries/highlights.scm"
wasm_out="${vs_dir}/assets/tree-sitter-snail.wasm"

if ! command -v tree-sitter >/dev/null 2>&1; then
  echo "tree-sitter CLI is required. Install it and retry." >&2
  exit 1
fi

if [ ! -f "${query_src}" ]; then
  echo "Missing Tree-sitter highlights at ${query_src}" >&2
  exit 1
fi

mkdir -p "${vs_dir}/assets" "${vs_dir}/queries"

cp "${query_src}" "${query_dest}"

build_args=("build" "--wasm" "-o" "${wasm_out}" "${ts_dir}")
if [ "${SNAIL_TS_DOCKER:-}" = "1" ]; then
  build_args=("build" "--wasm" "--docker" "-o" "${wasm_out}" "${ts_dir}")
fi

tree-sitter "${build_args[@]}"
echo "Wrote ${wasm_out}"
