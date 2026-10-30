# Releasing

A release is one tag push. `.github/workflows/release.yml` then:

1. renders the notes for that tag with git-cliff (`cliff.toml`) and publishes a GitHub Release, and
2. for stable `vX.Y.Z` tags, regenerates `PKGBUILD` and `.SRCINFO` from `packaging/aur/PKGBUILD` and pushes them to the `zenith-bar` AUR package.

Pre-release tags such as `v2.1.0-rc.1` produce a GitHub pre-release and leave the AUR alone.

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

Run the **Release** workflow by hand (Actions → Release → Run workflow) with an existing tag and `dry_run` ticked. It uploads the release notes, `PKGBUILD` and `.SRCINFO` as artifacts and publishes nothing.

Locally (Arch, needs `git-cliff`, `makepkg`, `namcap`):

```sh
.github/scripts/test_release_notes.sh
.github/scripts/test_generate_aur_files.sh --build   # builds the package from HEAD
```

## One-time setup

The AUR push needs the repository secret `AUR_SSH_PRIVATE_KEY`: the private half of an SSH key registered on the AUR account that maintains `zenith-bar`, either as-is or base64-encoded.
