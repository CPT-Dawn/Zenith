#!/usr/bin/env bash
# Writes PKGBUILD, zenith-bar.install and .SRCINFO for the zenith-bar AUR package into the
# current directory, from the template in packaging/aur/PKGBUILD.
#
#   generate_aur_files.sh --tag v2.0.0 [--tarball FILE]
#
# Without --tarball the GitHub tag archive is downloaded to compute its
# checksum. PKGREL (default 1) sets pkgrel. Needs makepkg (Arch) for .SRCINFO.
# Only stable vX.Y.Z tags are accepted: pre-releases don't go to the AUR.
set -euo pipefail

usage() { echo "usage: $0 --tag vX.Y.Z [--tarball FILE]" >&2; exit 2; }

tag=""
tarball=""
while (($#)); do
  case "$1" in
    --tag) tag="${2:-}"; shift 2 ;;
    --tarball) tarball="${2:-}"; shift 2 ;;
    *) usage ;;
  esac
done
[[ -n "$tag" ]] || usage

if [[ ! "$tag" =~ ^v([0-9]+\.[0-9]+\.[0-9]+)$ ]]; then
  echo "Refusing tag '$tag': expected a stable vX.Y.Z tag." >&2
  exit 1
fi
pkgver="${BASH_REMATCH[1]}"
pkgrel="${PKGREL:-1}"
repo="${GITHUB_REPOSITORY:-CPT-Dawn/Zenith}"
server_url="${GITHUB_SERVER_URL:-https://github.com}"
packaging="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)/packaging/aur"
template="$packaging/PKGBUILD"

if [[ -z "$tarball" ]]; then
  tarball="$(mktemp)"
  trap 'rm -f "$tarball"' EXIT
  curl -fsSL "${server_url}/${repo}/archive/refs/tags/${tag}.tar.gz" -o "$tarball"
fi
sha256="$(sha256sum "$tarball" | cut -d' ' -f1)"

sed -e "s/^pkgver=.*/pkgver=${pkgver}/" \
    -e "s/^pkgrel=.*/pkgrel=${pkgrel}/" \
    -e "s/^sha256sums=.*/sha256sums=('${sha256}')/" \
    -e "s|^url=.*|url=\"${server_url}/${repo}\"|" \
    "$template" >PKGBUILD

cp "$packaging/zenith-bar.install" zenith-bar.install
makepkg --printsrcinfo >.SRCINFO
