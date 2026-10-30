# Configuration

Zenith reads one TOML file. Its path is the first of:

1. `--config <FILE>` (`-c`)
2. `$ZENITH_BAR_CONFIG`
3. `$XDG_CONFIG_HOME/zenith-bar/config.toml` (usually `~/.config/zenith-bar/config.toml`)

Every key is optional. An empty or missing file gives the default look.

## Start a config: `zenith-bar init`

```sh
zenith-bar init            # writes ~/.config/zenith-bar/config.toml
zenith-bar init -c my.toml # writes somewhere else
zenith-bar init --force    # replaces an existing file
```

`init` writes a commented starter config whose values are the defaults, creating
the directory if needed. It never replaces an existing file unless you pass
`--force`.

## Validate a config: `zenith-bar check`

```sh
zenith-bar check
zenith-bar check -c my.toml
```

`check` prints every problem, not just the first, as `file:line:column: message`,
with a suggestion when a name looks like a typo. For this file:

```toml
[bar]
heigth = 30

[layout]
center = ["clok"]
```

`check` prints:

```text
config.toml:2:1: unknown key `heigth` in [bar] (did you mean `height`?)
config.toml:5:11: unknown module `clok` (did you mean `clock`?)
2 problems in config.toml
```

Lines and columns are 1-based. Diagnostics go to stderr. The exit code is `0` when the
file is valid and `1` when it has problems or cannot be read, so `check` works in
scripts, pre-commit hooks and AI agent loops.

## Editor autocomplete: the JSON Schema

The config's JSON Schema (draft-07) is generated from the config types and
committed at [`schema/config.schema.json`](../schema/config.schema.json). It is
published at:

```text
https://raw.githubusercontent.com/CPT-Dawn/Zenith/main/schema/config.schema.json
```

It gives key completion, hover docs, enum values and unknown-key warnings in any
editor that uses [Taplo](https://taplo.tamasfe.dev/), the TOML language server:
VS Code with *Even Better TOML*, Neovim, Helix, Zed and others.

### Option 1: the `#:schema` line (recommended)

Configs written by `zenith-bar init` already start with it. For an older config, add
this as the **first line**:

```toml
#:schema https://raw.githubusercontent.com/CPT-Dawn/Zenith/main/schema/config.schema.json
```

It is a plain TOML comment, so Zenith ignores it.

### Option 2: a Taplo config

Put a `.taplo.toml` (or `taplo.toml`) in `~/.config/zenith-bar/` or any parent
directory your editor opens:

```toml
[[rule]]
include = ["**/zenith-bar/config.toml"]

[rule.schema]
path = "https://raw.githubusercontent.com/CPT-Dawn/Zenith/main/schema/config.schema.json"
```

### Option 3: VS Code settings

With *Even Better TOML* installed, add to `settings.json`:

```json
"evenBetterToml.schema.associations": {
  ".*/zenith-bar/config\\.toml$": "https://raw.githubusercontent.com/CPT-Dawn/Zenith/main/schema/config.schema.json"
}
```

Offline, point any of these at a local copy of the schema instead
(`path = "file:///path/to/config.schema.json"`).

## For contributors: keeping the schema in sync

`tests/schema_drift.rs` fails when the committed schema differs from the generated
one. After changing any config type (or its doc comments, which become the
schema's descriptions), regenerate it:

```sh
UPDATE_SCHEMA=1 cargo test --test schema_drift
```
