#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Reset Neovim user data/config to a clean state.

This removes:
  - $XDG_CONFIG_HOME/nvim   (default: ~/.config/nvim)
  - $XDG_DATA_HOME/nvim     (default: ~/.local/share/nvim)
  - $XDG_STATE_HOME/nvim    (default: ~/.local/state/nvim)
  - $XDG_CACHE_HOME/nvim    (default: ~/.cache/nvim)

Usage:
  reset-neovim.sh [--force]

Behavior:
  - Default mode is dry-run and only prints the `rm -rf` commands.
  - `--force` executes the removals.
EOF
}

force=no

if (($# > 1)); then
  usage >&2
  exit 2
fi

if (($# == 1)); then
  if [[ "$1" == "--force" ]]; then
    force=yes
  else
    echo "Unknown option: $1" >&2
    echo >&2
    usage >&2
    exit 2
  fi
fi

config_home="${XDG_CONFIG_HOME:-$HOME/.config}"
data_home="${XDG_DATA_HOME:-$HOME/.local/share}"
state_home="${XDG_STATE_HOME:-$HOME/.local/state}"
cache_home="${XDG_CACHE_HOME:-$HOME/.cache}"

paths=(
  "$config_home/nvim"
  "$data_home/nvim"
  "$state_home/nvim"
  "$cache_home/nvim"
)

echo "Neovim reset targets:"
for p in "${paths[@]}"; do
  echo "  - $p"
done

echo
if [[ "$force" != "yes" ]]; then
  echo "Dry-run mode (no changes):"
  for p in "${paths[@]}"; do
    if [[ -e "$p" ]]; then
      printf 'rm -rf -- %q\n' "$p"
    else
      echo "# skip missing: $p"
    fi
  done
  echo
  echo "Re-run with --force to execute."
  exit 0
fi

echo "Force mode: executing reset..."
for p in "${paths[@]}"; do
  if [[ ! -e "$p" ]]; then
    echo "skip missing: $p"
    continue
  fi
  printf 'rm -rf -- %q\n' "$p"
  rm -rf -- "$p"
done

echo "Neovim reset complete."
