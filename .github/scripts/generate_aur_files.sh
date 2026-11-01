#!/usr/bin/env bash
# Writes PKGBUILD, zenith-bar.install and .SRCINFO for one of the AUR packages
# into the current directory, from its template:
#
#   zenith-bar      packaging/aur/PKGBUILD      builds from the tag archive
#   zenith-bar-bin  packaging/aur-bin/PKGBUILD  repackages the release tarball
#
#   generate_aur_files.sh --tag v2.0.0 [--package NAME] [--tarball FILE]
#
# --package defaults to zenith-bar. Without --tarball the source (the GitHub
# tag archive, or the release asset for -bin) is downloaded to compute its
# checksum. PKGREL (default 1) sets pkgrel. Needs makepkg (Arch) for .SRCINFO.
# Only stable vX.Y.Z tags are accepted: pre-releases don't go to the AUR.
set -euo pipefail

usage() { echo "usage: $0 --tag vX.Y.Z [--package zenith-bar|zenith-bar-bin] [--tarball FILE]" >&2; exit 2; }

tag=""
package="zenith-bar"
tarball=""
while (($#)); do
  case "$1" in
    --tag) tag="${2:-}"; shift 2 ;;
    --package) package="${2:-}"; shift 2 ;;
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
packaging_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)/packaging"

case "$package" in
  zenith-bar)
    template="$packaging_root/aur/PKGBUILD"
    source_url="${server_url}/${repo}/archive/refs/tags/${tag}.tar.gz"
    ;;
  zenith-bar-bin)
    template="$packaging_root/aur-bin/PKGBUILD"
    source_url="${server_url}/${repo}/releases/download/${tag}/zenith-bar-${pkgver}-x86_64.tar.gz"
    ;;
  *)
    echo "Unknown package '$package'." >&2
    usage
    ;;
esac

if [[ -z "$tarball" ]]; then
  tarball="$(mktemp)"
  trap 'rm -f "$tarball"' EXIT
  curl -fsSL "$source_url" -o "$tarball"
fi
sha256="$(sha256sum "$tarball" | cut -d' ' -f1)"

# sha256sums and sha256sums_x86_64 alike.
sed -e "s/^pkgver=.*/pkgver=${pkgver}/" \
    -e "s/^pkgrel=.*/pkgrel=${pkgrel}/" \
    -e "s/^\(sha256sums[_a-z0-9]*\)=.*/\1=('${sha256}')/" \
    -e "s|^url=.*|url=\"${server_url}/${repo}\"|" \
    "$template" >PKGBUILD

cp "$packaging_root/aur/zenith-bar.install" zenith-bar.install
makepkg --printsrcinfo >.SRCINFO
