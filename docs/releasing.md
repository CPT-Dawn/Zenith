# Releasing

A release is one tag push. `.github/workflows/release.yml` then:

1. renders the notes for that tag with git-cliff (`cliff.toml`) and publishes a GitHub Release,
2. tests and builds the release binary in an Arch container, packs it with `.github/scripts/package_binary.sh` into `zenith-bar-X.Y.Z-x86_64.tar.gz` (plus `.sha256`), attests its build provenance and attaches both to the release, and
3. for stable `vX.Y.Z` tags, regenerates `PKGBUILD` and `.SRCINFO` for both AUR packages and pushes them:
   - `zenith-bar` from `packaging/aur/PKGBUILD`, building from the tag archive;
   - `zenith-bar-bin` from `packaging/aur-bin/PKGBUILD`, repackaging the tarball from step 2.

Pre-release tags such as `v2.1.0-rc.1` produce a GitHub pre-release (with the tarball) and leave the AUR alone.

Keep the `depends` arrays of the two PKGBUILDs identical; the packaging tests fail if they drift.

## Steps

```sh
# 1. Bump the version in Cargo.toml (no "v"), refresh the lockfile.
cargo update -p zenith-bar
# 2. Regenerate the changelog for the new tag.
git cliff --tag v2.0.0 -o CHANGELOG.md
git commit -am "chore(release): prepare for v2.0.0"
# 3. Tag and push.
git tag -s v2.0.0 -m "Zenith 2.0.0"
git push origin main v2.0.0
```

The workflow refuses a tag that doesn't match the `Cargo.toml` version.

## Previewing

Run the **Release** workflow by hand (Actions → Release → Run workflow) with an existing tag and `dry_run` ticked. It uploads the release notes, the binary tarball, and each package's `PKGBUILD` and `.SRCINFO` as artifacts and publishes nothing.

Locally (Arch, needs `git-cliff`, `makepkg`, `namcap`):

```sh
.github/scripts/test_release_notes.sh
.github/scripts/test_generate_aur_files.sh --build   # builds both packages from HEAD
```

## One-time setup

The AUR push needs the repository secret `AUR_SSH_PRIVATE_KEY`: the private half of an SSH key registered on the AUR account that maintains `zenith-bar` and `zenith-bar-bin`, either as-is or base64-encoded. The same key creates `zenith-bar-bin` on its first push.
