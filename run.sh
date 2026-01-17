#!/usr/bin/env bash
set -euo pipefail

uv run -- python -m maturin develop
exec uv run -- snail "$@"
