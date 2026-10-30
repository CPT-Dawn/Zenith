#!/usr/bin/env bash
# Tests cliff.toml: builds a scratch repo with known commits, renders the
# release notes for its latest tag the way the release workflow does, and
# checks what readers will see.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

failures=0
fail() { echo "FAIL: $*" >&2; failures=$((failures + 1)); }
pass() { echo "ok:   $*"; }
has() { if grep -qF -- "$2" "$1"; then pass "$3"; else fail "$3 (missing '$2')"; fi; }
lacks() { if grep -qF -- "$2" "$1"; then fail "$3 (found '$2')"; else pass "$3"; fi; }

g() { git -C "$work/repo" -c user.name=T -c user.email=t@example.com -c commit.gpgsign=false -c tag.gpgsign=false "$@"; }
mkdir "$work/repo"
g init -q
c() { g commit -q --allow-empty -m "$1"; }

c "feat: a v1 feature"
g tag v1.2.0
c "feat: the old feature"
g tag v2.0.0
c "add demo image"
c "feat(clock): tick on the second boundary"
c "fix(media): stop ghost players haunting the bar"
c "perf(ui): skip redraws nobody asked for"
c "docs: explain the glass"
c "feat!: raze v1"
c "chore(release): prepare for v2.1.0"
c "ci: poke the runner"
g tag v2.1.0

# The release workflow checks out the tag and renders `--current`.
notes="$work/notes.md"
(cd "$work/repo" && git-cliff --config "$repo_root/cliff.toml" --current --strip header) >"$notes"
cat "$notes"
echo "---"

has "$notes" "## [2.1.0]" "heading names the version without the v"
has "$notes" "### Features" "features are grouped"
has "$notes" "### Bug Fixes" "fixes are grouped"
has "$notes" "### Performance" "perf commits are grouped"
has "$notes" "### Documentation" "docs are grouped"
has "$notes" "**clock:** Tick on the second boundary" "scope is shown and subject capitalised"
has "$notes" "[**breaking**] Raze v1" "breaking changes are flagged"
lacks "$notes" "the old feature" "only the latest release is included"
lacks "$notes" "demo image" "unconventional commits are dropped"
lacks "$notes" "prepare for v2.1.0" "release chores are dropped"
lacks "$notes" "poke the runner" "ci noise is dropped"

# Re-running the workflow for an older tag renders that tag, not the newest.
old="$work/old.md"
g checkout -q v2.0.0
(cd "$work/repo" && git-cliff --config "$repo_root/cliff.toml" --current --strip header) >"$old"
has "$old" "## [2.0.0]" "an older tag gets its own heading"
has "$old" "The old feature" "an older tag gets its own commits"
lacks "$old" "Raze v1" "an older tag excludes later commits"
g checkout -q v2.1.0

full="$work/CHANGELOG.md"
(cd "$work/repo" && git-cliff --config "$repo_root/cliff.toml") >"$full"
has "$full" "# Changelog" "full changelog has a header"
has "$full" "## [2.0.0]" "full changelog covers older releases"
lacks "$full" "a v1 feature" "v1 history is left out"
lacks "$full" "## [1.2.0]" "v1 releases are left out"

echo
if ((failures)); then
  echo "$failures check(s) failed" >&2
  exit 1
fi
echo "all checks passed"
