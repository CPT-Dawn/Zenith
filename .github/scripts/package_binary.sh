#!/usr/bin/env bash
# Packs a built zenith-bar binary into the release tarball that the
# zenith-bar-bin AUR package installs, plus a .sha256 next to it.
#
#   package_binary.sh --version 2.1.0 --binary target/release/zenith-bar [--out DIR]
#
# The tarball is zenith-bar-<version>-x86_64.tar.gz with a single top
# directory holding zenith-bar, LICENSE and README.md. It is reproducible:
# entries are sorted, owned by root and dated SOURCE_DATE_EPOCH (default 0).
# Prints the tarball path.
set -euo pipefail

usage() { echo "usage: $0 --version X.Y.Z --binary FILE [--out DIR]" >&2; exit 2; }

version=""
binary=""
out="."
while (($#)); do
  case "$1" in
    --version) version="${2:-}"; shift 2 ;;
    --binary) binary="${2:-}"; shift 2 ;;
    --out) out="${2:-}"; shift 2 ;;
    *) usage ;;
  esac
done
[[ -n "$version" && -n "$binary" ]] || usage
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]] || {
  echo "Refusing version '$version': expected X.Y.Z (no v)." >&2
  exit 1
}
[[ -f "$binary" ]] || { echo "No binary at $binary" >&2; exit 1; }

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
name="zenith-bar-${version}-x86_64"
mkdir -p "$out"
out="$(cd "$out" && pwd)"

stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
mkdir "$stage/$name"
install -m755 "$binary" "$stage/$name/zenith-bar"
install -m644 "$repo_root/LICENSE" "$repo_root/README.md" "$stage/$name/"

tarball="$out/$name.tar.gz"
tar -C "$stage" --sort=name --owner=0 --group=0 --numeric-owner \
    --mtime="@${SOURCE_DATE_EPOCH:-0}" -cf - "$name" | gzip -9n >"$tarball"
(cd "$out" && sha256sum "$name.tar.gz" >"$name.tar.gz.sha256")
echo "$tarball"
