#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

if ! command -v jj >/dev/null 2>&1; then
  echo "jj is required but not found in PATH" >&2
  exit 1
fi

if ! command -v mise >/dev/null 2>&1; then
  echo "mise is required but not found in PATH" >&2
  exit 1
fi

if ! jj root >/dev/null 2>&1; then
  if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    echo "Initializing colocated jj repo (jj git init --colocate)"
    jj git init --colocate >/dev/null
  else
    echo "No jj repo found (and not in a git repo to colocate)" >&2
    exit 1
  fi
fi

jj config set --repo aliases.gate-fast '["util", "exec", "--", "mise", "run", "hk-fix"]'
jj config set --repo aliases.gate-fix '["util", "exec", "--", "mise", "run", "hk-fix"]'
jj config set --repo aliases.gate-check '["util", "exec", "--", "mise", "run", "hk-check"]'
jj config set --repo aliases.gate-pre-commit '["util", "exec", "--", "mise", "run", "hk-pre-commit"]'

echo "Installed repo-local jj aliases:"
echo "  jj gate-fast         # fast local fixes/hygiene (hk fix: all files, no stage)"
echo "  jj gate-fix          # fast local fixes/hygiene (hk fix: all files, no stage)"
echo "  jj gate-check        # full baseline closure gate"
echo "  jj gate-pre-commit   # git-staged pre-commit profile"
