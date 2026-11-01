#!/usr/bin/env bash
# Tests for generate_aur_files.sh. Needs an Arch environment (makepkg).
#
#   .github/scripts/test_generate_aur_files.sh          # fast checks
#   .github/scripts/test_generate_aur_files.sh --build  # also build and inspect the package
#
# The tarball under test is `git archive HEAD`, so commit before running.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
generator="$repo_root/.github/scripts/generate_aur_files.sh"
run_build=false
[[ "${1:-}" == "--build" ]] && run_build=true

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

failures=0
fail() { echo "FAIL: $*" >&2; failures=$((failures + 1)); }
pass() { echo "ok:   $*"; }

assert_contains() { # file line description
  if grep -qxF -- "$2" "$1"; then pass "$3"; else fail "$3 (missing '$2' in $(basename "$1"))"; fi
}
assert_absent() { # file pattern description
  if grep -q -- "$2" "$1"; then fail "$3 (found '$2' in $(basename "$1"))"; else pass "$3"; fi
}

# A tarball shaped like GitHub's tag archive: top directory Zenith-<version>.
tarball="$work/zenith-bar-2.0.0.tar.gz"
git -C "$repo_root" archive --format=tar.gz --prefix=Zenith-2.0.0/ -o "$tarball" HEAD
expected_sha="$(sha256sum "$tarball" | cut -d' ' -f1)"

# --- a stable tag produces a PKGBUILD and .SRCINFO for that version ---------
out="$work/aur"
mkdir -p "$out"
(cd "$out" && bash "$generator" --tag v2.0.0 --tarball "$tarball")

srcinfo="$out/.SRCINFO"
pkgbuild="$out/PKGBUILD"
[[ -f "$pkgbuild" ]] && pass "PKGBUILD written" || fail "PKGBUILD written"
[[ -f "$srcinfo" ]] && pass ".SRCINFO written" || fail ".SRCINFO written"

assert_contains "$srcinfo" "pkgbase = zenith-bar" "package is named zenith-bar"
assert_contains "$srcinfo" $'\tpkgver = 2.0.0' "pkgver comes from the tag"
assert_contains "$srcinfo" $'\tpkgrel = 1' "pkgrel defaults to 1"
assert_contains "$srcinfo" $'\tsha256sums = '"$expected_sha" "checksum matches the tarball"
assert_contains "$srcinfo" \
  $'\tsource = zenith-bar-2.0.0.tar.gz::https://github.com/CPT-Dawn/Zenith/archive/refs/tags/v2.0.0.tar.gz' \
  "source is the GitHub tag archive"
assert_contains "$srcinfo" $'\tdepends = gtk4>=1:4.22' "requires GTK >= 4.22"
assert_contains "$srcinfo" $'\tdepends = gtk4-layer-shell' "requires gtk4-layer-shell"
assert_contains "$srcinfo" $'\tmakedepends = cargo' "builds with cargo"
assert_absent "$srcinfo" "playerctl" "playerctl dependency is gone"
assert_contains "$pkgbuild" "pkgver=2.0.0" "PKGBUILD carries the version"
assert_contains "$srcinfo" $'\tdepends = pango' "declares pango, which the binary links"
assert_contains "$srcinfo" $'\tdepends = graphene' "declares graphene, which the binary links"

# --- the install script warns 1.x users about the rename ---------------------
install_file="$out/zenith-bar.install"
[[ -f "$install_file" ]] && pass "zenith-bar.install written" || fail "zenith-bar.install written"
assert_contains "$srcinfo" $'\tinstall = zenith-bar.install' ".SRCINFO declares the install script"
notice() { # old-version -> what post_upgrade prints
  bash -c 'source "$1"; post_upgrade 2.0.0-1 "$2"' _ "$install_file" "$1" 2>&1
}
if notice 1.2.0-1 | grep -q "zenith-bar"; then pass "upgrade from 1.x prints the rename notice"; else fail "upgrade from 1.x prints the rename notice"; fi
if notice 1.2.0-1 | grep -q "migrating-from-v1"; then pass "the notice links the migration guide"; else fail "the notice links the migration guide"; fi
if [[ -z "$(notice 2.0.0-1)" ]]; then pass "upgrade within 2.x stays quiet"; else fail "upgrade within 2.x stays quiet"; fi

# The generated .SRCINFO must be exactly what makepkg derives from the PKGBUILD.
if diff <(cd "$out" && makepkg --printsrcinfo) "$srcinfo" >/dev/null; then
  pass ".SRCINFO is in sync with PKGBUILD"
else
  fail ".SRCINFO is in sync with PKGBUILD"
fi

# --- PKGREL override ---------------------------------------------------------
out2="$work/aur-rel"
mkdir -p "$out2"
(cd "$out2" && PKGREL=3 bash "$generator" --tag v2.0.0 --tarball "$tarball")
assert_contains "$out2/.SRCINFO" $'\tpkgrel = 3' "PKGREL overrides pkgrel"

# --- bad tags are refused -----------------------------------------------------
for bad in 2.0.0 v2.0.0-rc.1 v2.0 vnext; do
  dir="$work/bad-$bad"
  mkdir -p "$dir"
  if (cd "$dir" && bash "$generator" --tag "$bad" --tarball "$tarball") >/dev/null 2>&1; then
    fail "tag '$bad' is refused"
  else
    pass "tag '$bad' is refused"
  fi
done

for bad in zenith zenith-bar-git; do
  dir="$work/badpkg-$bad"
  mkdir -p "$dir"
  if (cd "$dir" && bash "$generator" --tag v2.0.0 --package "$bad" --tarball "$tarball") >/dev/null 2>&1; then
    fail "package '$bad' is refused"
  else
    pass "package '$bad' is refused"
  fi
done

# --- the release tarball for zenith-bar-bin -----------------------------------
packer="$repo_root/.github/scripts/package_binary.sh"
fake_bin="$work/fake-zenith-bar"
printf '#!/bin/sh\necho zenith-bar 2.0.0\n' >"$fake_bin"
bin_tarball="$(bash "$packer" --version 2.0.0 --binary "$fake_bin" --out "$work/dist")"
[[ "$(basename "$bin_tarball")" == "zenith-bar-2.0.0-x86_64.tar.gz" ]] \
  && pass "release tarball is named zenith-bar-<ver>-x86_64.tar.gz" \
  || fail "release tarball is named zenith-bar-<ver>-x86_64.tar.gz (got $bin_tarball)"
bsdtar -tvf "$bin_tarball" >"$work/bin-listing"
for entry in zenith-bar LICENSE README.md; do
  if grep -q " zenith-bar-2.0.0-x86_64/$entry\$" "$work/bin-listing"; then
    pass "release tarball holds $entry"
  else
    fail "release tarball holds $entry"
  fi
done
if grep -q '^-rwxr-xr-x .* zenith-bar-2.0.0-x86_64/zenith-bar$' "$work/bin-listing"; then
  pass "release binary is executable"
else
  fail "release binary is executable"
fi
if (cd "$work/dist" && sha256sum -c --quiet zenith-bar-2.0.0-x86_64.tar.gz.sha256); then
  pass "release .sha256 matches"
else
  fail "release .sha256 matches"
fi
again="$(bash "$packer" --version 2.0.0 --binary "$fake_bin" --out "$work/dist-again")"
if cmp -s "$bin_tarball" "$again"; then pass "release tarball is reproducible"; else fail "release tarball is reproducible"; fi
if bash "$packer" --version v2.0.0 --binary "$fake_bin" --out "$work/dist-bad" >/dev/null 2>&1; then
  fail "packer refuses a v-prefixed version"
else
  pass "packer refuses a v-prefixed version"
fi

# --- zenith-bar-bin: PKGBUILD and .SRCINFO -----------------------------------
bin_out="$work/aur-bin"
mkdir -p "$bin_out"
(cd "$bin_out" && bash "$generator" --tag v2.0.0 --package zenith-bar-bin --tarball "$bin_tarball")
bin_srcinfo="$bin_out/.SRCINFO"
bin_sha="$(sha256sum "$bin_tarball" | cut -d' ' -f1)"
assert_contains "$bin_srcinfo" "pkgbase = zenith-bar-bin" "bin package is named zenith-bar-bin"
assert_contains "$bin_srcinfo" $'\tpkgver = 2.0.0' "bin pkgver comes from the tag"
assert_contains "$bin_srcinfo" $'\tsha256sums_x86_64 = '"$bin_sha" "bin checksum matches the release tarball"
assert_contains "$bin_srcinfo" \
  $'\tsource_x86_64 = zenith-bar-2.0.0-x86_64.tar.gz::https://github.com/CPT-Dawn/Zenith/releases/download/v2.0.0/zenith-bar-2.0.0-x86_64.tar.gz' \
  "bin source is the GitHub Release asset"
assert_contains "$bin_srcinfo" $'\tprovides = zenith-bar=2.0.0' "bin provides zenith-bar"
assert_contains "$bin_srcinfo" $'\tconflicts = zenith-bar' "bin conflicts with zenith-bar"
assert_absent "$bin_srcinfo" "makedepends" "bin needs no build tools"
assert_contains "$bin_srcinfo" $'\tinstall = zenith-bar.install' "bin carries the install script"
# Runtime dependencies must match the source package exactly.
if diff <(grep -P '^\t(depends|optdepends) = ' "$srcinfo") \
        <(grep -P '^\t(depends|optdepends) = ' "$bin_srcinfo") >/dev/null; then
  pass "bin and source packages share runtime dependencies"
else
  fail "bin and source packages share runtime dependencies"
fi
if diff <(cd "$bin_out" && makepkg --printsrcinfo) "$bin_srcinfo" >/dev/null; then
  pass "bin .SRCINFO is in sync with PKGBUILD"
else
  fail "bin .SRCINFO is in sync with PKGBUILD"
fi

# --- full build: the package installs /usr/bin/zenith-bar --------------------
if $run_build; then
  build="$work/build"
  mkdir -p "$build"
  cp "$pkgbuild" "$install_file" "$build/"
  cp "$tarball" "$build/" # pre-seeded, so makepkg doesn't download it
  # Pin PKGDEST/SRCDEST so a user's makepkg.conf can't move the results.
  export PKGDEST="$build" SRCDEST="$build"
  (cd "$build" && makepkg --force --nodeps --noconfirm >"$work/makepkg.log" 2>&1) \
    || { tail -40 "$work/makepkg.log" >&2; fail "makepkg builds the package"; }
  pkgfile="$(cd "$build" && makepkg --packagelist | grep -v -- '-debug-' | head -1)"
  if [[ ! -f "$pkgfile" ]]; then
    fail "makepkg produced $pkgfile"
  else
    pass "makepkg builds the package"
    listing="$work/listing"
    bsdtar -tf "$pkgfile" >"$listing"
    assert_contains "$listing" "usr/bin/zenith-bar" "installs the zenith-bar binary"
    assert_contains "$listing" "usr/share/licenses/zenith-bar/LICENSE" "installs the licence"
    assert_absent "$listing" "usr/bin/zenith$" "no v1 zenith binary"
    namcap "$pkgfile" | tee "$work/namcap.log"
    if grep -E ' E: ' "$work/namcap.log" >&2; then fail "namcap reports no errors"; else pass "namcap reports no errors"; fi
    if grep -E 'implicitly satisfied' "$work/namcap.log" >&2; then fail "no implicit dependencies"; else pass "no implicit dependencies"; fi
    if bsdtar -xOf "$pkgfile" .INSTALL 2>/dev/null | grep -q post_upgrade; then pass "package carries the install script"; else fail "package carries the install script"; fi

    # --- zenith-bar-bin from the binary just built --------------------------
    real_bin="$build/src/Zenith-2.0.0/target/release/zenith-bar"
    bin_build="$work/build-bin"
    mkdir -p "$bin_build"
    real_tarball="$(bash "$packer" --version 2.0.0 --binary "$real_bin" --out "$bin_build")"
    (cd "$bin_build" && bash "$generator" --tag v2.0.0 --package zenith-bar-bin --tarball "$real_tarball")
    export PKGDEST="$bin_build" SRCDEST="$bin_build"
    (cd "$bin_build" && makepkg --force --nodeps --noconfirm >"$work/makepkg-bin.log" 2>&1) \
      || { tail -40 "$work/makepkg-bin.log" >&2; fail "makepkg builds the bin package"; }
    bin_pkgfile="$(cd "$bin_build" && makepkg --packagelist | head -1)"
    if [[ ! -f "$bin_pkgfile" ]]; then
      fail "makepkg produced $bin_pkgfile"
    else
      pass "makepkg builds the bin package"
      bsdtar -tf "$bin_pkgfile" >"$work/bin-pkg-listing"
      assert_contains "$work/bin-pkg-listing" "usr/bin/zenith-bar" "bin installs the zenith-bar binary"
      assert_contains "$work/bin-pkg-listing" "usr/share/licenses/zenith-bar-bin/LICENSE" "bin installs the licence"
      if cmp -s "$real_bin" <(bsdtar -xOf "$bin_pkgfile" usr/bin/zenith-bar); then
        pass "bin ships the release binary byte for byte"
      else
        fail "bin ships the release binary byte for byte"
      fi
      namcap "$bin_pkgfile" | tee "$work/namcap-bin.log"
      if grep -E ' E: ' "$work/namcap-bin.log" >&2; then fail "namcap reports no errors for bin"; else pass "namcap reports no errors for bin"; fi
      if grep -E 'implicitly satisfied' "$work/namcap-bin.log" >&2; then fail "no implicit dependencies in bin"; else pass "no implicit dependencies in bin"; fi
    fi
  fi
fi

echo
if ((failures)); then
  echo "$failures check(s) failed" >&2
  exit 1
fi
echo "all checks passed"
