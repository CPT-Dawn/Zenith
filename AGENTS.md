# Zenith

## Agent skills

### Issue tracker

Issues live in GitHub Issues for CPT-Dawn/Zenith, managed via the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Default vocabulary: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `GLOSSARY.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.

## Commands

- Build: `cargo build --release` (binary: `target/release/zenith-bar`)
- Test: `cargo test --all-targets` (headless; no compositor needed)
- Lint: `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
- Config check: `target/release/zenith-bar check` / `zenith-bar init` (see `docs/configuration.md`)
- Run on the desktop: the user's bar is `/usr/bin/zenith-bar` started by Hyprland (Lua config) and is single-instance, so stop it first, run `target/release/zenith-bar`, then restore it with `hyprctl eval 'hl.exec_cmd("zenith-bar")'`.
- Isolated live runs (crash repros, hotplug): headless Sway (`WLR_BACKENDS=headless`) on a private `dbus-run-session` whose config has **no** service dirs, with a private short `XDG_RUNTIME_DIR` (socket paths max 108 bytes) and `GDK_DEBUG=no-portals NO_AT_BRIDGE=1`; otherwise activated portals/AT-SPI clobber the user's session sockets in `/run/user/1000`. `bench/` (`zenith-bench --smoke`) does the same in CI.
- Release: `docs/releasing.md` (bump `Cargo.toml`, `cargo update -p zenith-bar`, `git cliff --tag vX.Y.Z -o CHANGELOG.md`, signed tag, push; the Release workflow publishes GitHub + both AUR packages).
