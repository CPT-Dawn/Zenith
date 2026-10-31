# Contributing to Zenith

Thanks for helping. Zenith has one promise: **the lightest beautiful status bar for Wayland**. Every change is weighed against both halves of that sentence, so a pretty feature that keeps the bar awake, or a lean one that looks rough, will get pushback.

## Reporting bugs and asking for features

Use the [issue forms](https://github.com/CPT-Dawn/Zenith/issues/new/choose). For bugs, `zenith-bar --version`, your compositor and version, and the output of `zenith-bar check` save a round trip. `ZENITH_BAR_LOG=debug zenith-bar` gives detailed logs.

New modules (workspaces, battery, network, tray, …) are deliberately out of scope for v2. Requests are still welcome: they are how we decide what comes next.

## Building

You need GTK 4.22+, gtk4-layer-shell and Rust 1.92+ (see [installation](docs/installation.md#from-source)).

```sh
cargo build
cargo run -- --config my-test.toml   # run against a scratch config, not your own
```

## Before you open a PR

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

If you changed anything that could affect idle cost (timers, D-Bus, CSS, animations), also run the benchmark harness on a release build and compare against [docs/performance.md](docs/performance.md):

```sh
cargo build --release
cargo bench-bar --bin target/release/zenith-bar --preset bloom --budget bench/budget.toml --smoke
```

CI runs the same harness on every preset and fails when a **Performance budget** is exceeded.

## How the code is tested

Tests check behaviour you can see from outside, at three seams:

1. **Module models**: typed inputs in, view state out. Models know nothing about GTK; views are thin binders and are not unit-tested.
2. **Config resolution**: TOML text in, a resolved config or exact diagnostics (line, column, suggestion) out.
3. **The benchmark harness**: a real bar on a headless Sway, measured.

Don't mock D-Bus or assert on widget trees or private helpers. A test should survive a refactor that keeps behaviour the same.

## Keeping docs honest

Several docs are checked by tests, so update them in the same PR:

- **Config keys**: after changing a config type or its doc comments, regenerate the schema with `UPDATE_SCHEMA=1 cargo test --test schema_drift`, and update the reference in [docs/configuration.md](docs/configuration.md) (`tests/config_reference.rs` checks every key, default and example).
- **Theming**: every `--zb-*` variable and `zb-*` class must be listed in [docs/theming.md](docs/theming.md) (`tests/theming.rs`).
- **Numbers**: a resource number goes in the README or docs only if the harness produced it, with the method and hardware next to it. Never loosen a budget without new measurements.

## Design records

- [`GLOSSARY.md`](GLOSSARY.md) defines the domain words (Bar, Output, Module, Section, Theme, Preset, Focal element, Motion, …). Use them in code, tests and docs.
- [`docs/adr/`](docs/adr/) holds the architecture decisions. If your change goes against one, say so in the PR; if it should replace one, add a new ADR.

Two rules come up often:

- **One main loop, no helper processes** (ADR-0002). No threads, no tokio, no spawning programs except user-configured click commands.
- **No ambient animation by default** (ADR-0003). Anything that animates continuously belongs in the `ambient` stylesheet.

## Commits and PRs

- [Conventional commits](https://www.conventionalcommits.org/) (`feat(media): …`, `fix(clock): …`, `docs: …`). The changelog and release notes are generated from them by git-cliff, so the subject line is what users read.
- Keep the subject at 72 characters or fewer.
- One logical change per PR. Branch from `main` and fill in the PR template.

By contributing, you agree your work is released under the [MIT license](LICENSE).
