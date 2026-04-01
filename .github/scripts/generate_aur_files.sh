#!/usr/bin/env bash
set -euo pipefail

if [[ -z "${GITHUB_REF_NAME:-}" ]]; then
  echo "GITHUB_REF_NAME is required (example: v1.0.0)." >&2
  exit 1
fi

release_tag="${GITHUB_REF_NAME}"
if [[ "$release_tag" != v* ]]; then
  echo "Release tag must start with 'v' (received: $release_tag)." >&2
  exit 1
fi

pkgname="zenith-bar"
_reponame="Zenith"
pkgver="${release_tag#v}"
pkgrel="${PKGREL:-1}"
maintainer="${AUR_MAINTAINER:-CPT-Dawn <dawnsp0456@gmail.com>}"
repo="${GITHUB_REPOSITORY:-CPT-Dawn/Zenith}"
server_url="${GITHUB_SERVER_URL:-https://github.com}"

tarball_url="${server_url}/${repo}/archive/refs/tags/${release_tag}.tar.gz"

tmp_tarball="$(mktemp)"
trap 'rm -f "$tmp_tarball"' EXIT

curl -fsSL "$tarball_url" -o "$tmp_tarball"
sha256="$(sha256sum "$tmp_tarball" | awk '{print $1}')"

cat > PKGBUILD <<EOF
# Maintainer: ${maintainer}
pkgname=${pkgname}
_reponame=${_reponame}
pkgver=${pkgver}
pkgrel=${pkgrel}
pkgdesc="Sleek animated Wayland status bar for Hyprland in Rust"
arch=('x86_64')
url="${server_url}/${repo}"
license=('MIT')
depends=('gtk4' 'gtk4-layer-shell' 'glibc')
makedepends=('cargo')
optdepends=(
  'playerctl: media module support'
  'ttf-inter: recommended UI font'
  'ttf-jetbrains-mono-nerd: recommended icon and mono font'
)
source=("\${pkgname}-\${pkgver}.tar.gz::${tarball_url}")
sha256sums=('${sha256}')

prepare() {
  cd "\${_reponame}-\${pkgver}"
  export CARGO_HOME="\${srcdir}/cargo-home"
  cargo fetch --locked
}

build() {
  cd "\${_reponame}-\${pkgver}"
  export CARGO_HOME="\${srcdir}/cargo-home"
  export CARGO_TARGET_DIR="\${srcdir}/target"
  cargo build --release --locked --frozen
}

package() {
  cd "\${_reponame}-\${pkgver}"

  install -Dm755 "\${srcdir}/target/release/zenith" "\${pkgdir}/usr/bin/zenith"
  install -Dm644 LICENSE "\${pkgdir}/usr/share/licenses/\${pkgname}/LICENSE"
  install -Dm644 README.md "\${pkgdir}/usr/share/doc/\${pkgname}/README.md"
}
EOF

cat > .SRCINFO <<EOF
pkgbase = ${pkgname}
    pkgdesc = Sleek animated Wayland status bar for Hyprland in Rust
    pkgver = ${pkgver}
    pkgrel = ${pkgrel}
    url = ${server_url}/${repo}
    arch = x86_64
    license = MIT
    makedepends = cargo
    depends = gtk4
    depends = gtk4-layer-shell
    depends = glibc
    optdepends = playerctl: media module support
    optdepends = ttf-inter: recommended UI font
    optdepends = ttf-jetbrains-mono-nerd: recommended icon and mono font
    source = ${pkgname}-${pkgver}.tar.gz::${tarball_url}
    sha256sums = ${sha256}

pkgname = ${pkgname}
EOF