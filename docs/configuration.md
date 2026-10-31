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

Unknown keys, values of the wrong type and invalid formats are all reported in one run.
Two limits: a TOML syntax error stops the run at that point (nothing after it can be
read reliably), and inside a nested table such as `[system.temperature]` or a list,
only the first bad value is reported.

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

## Reference

Every key, grouped by table. **Default** is what you get when the key is left out;
"unset" means the key has no value unless you write one. **Example** is a valid
value you can paste after `key = `. A test (`tests/config_reference.rs`) checks
these tables against the config types, so they can't drift from what Zenith
actually accepts.

Unknown keys are errors, not silently ignored: `zenith-bar check` names them and
suggests the closest real key.

### `[bar]`

Placement, geometry and motion of every bar.

| Key | Default | Example | Description |
|---|---|---|---|
| `position` | `"top"` | `"bottom"` | Edge of the output the bar sits on: `"top"` or `"bottom"`. |
| `monitors` | `"all"` | `["DP-1", "HDMI-A-1"]` | Outputs that get a bar: `"all"`, or a list of connector names. Bars come and go as monitors are plugged in and out. |
| `height` | `34` | `30` | Bar height in pixels. |
| `margin` | `8` | `0` | Gap between the bar and the output edges, in pixels. `0` makes the bar flush; anything larger makes it float. |
| `radius` | `14` | `0` | Corner radius of the bar, in pixels (`--zb-bar-radius`). Module pills have their own: `[theme] module_radius`. |
| `motion` | `"subtle"` | `"off"` | Animation level: `"off"`, `"subtle"` (short transitions on events only) or `"ambient"` (continuous effects, at a cost; see [theming](theming.md#motion)). |
| `focal` | `"clock"` | `"media"` | The module that gets the neon glow, or `"none"`. It must be in `[layout]` if you set it; the default quietly steps aside when the clock isn't on the bar. |

### `[layout]`

Which **Modules** appear in each **Section**, in order. Available modules:
`logo`, `todo`, `clock`, `calendar`, `media`, `system`. A module may appear only
once across all sections; leave it out of every section to disable it. Leaving
`[layout]` out gives the default layout; once you write the table, any section
you leave out is empty.

| Key | Default | Example | Description |
|---|---|---|---|
| `left` | `["logo", "todo"]` | `["logo"]` | Modules in the left section, in order. |
| `center` | `["clock", "calendar"]` | `["clock"]` | Modules in the center section, in order. |
| `right` | `["media", "system"]` | `["system", "calendar"]` | Modules in the right section, in order. |

### `[clock]`

| Key | Default | Example | Description |
|---|---|---|---|
| `format` | `"%H:%M"` | `"%a %d %b  %H:%M"` | strftime-style format. The clock wakes on the minute boundary, or every second only if the format shows seconds (`%S`, `%T`, ...). |

### `[calendar]`

The date. Click it to open a month calendar.

| Key | Default | Example | Description |
|---|---|---|---|
| `format` | `"%a %d %b"` | `"%Y-%m-%d"` | strftime-style date format. The label is redrawn once a day, at local midnight. |

### `[media]`

What's playing, from any MPRIS player on the session bus (no `playerctl` needed).
Zenith follows the player that is playing, otherwise the most recent one. Click
to play/pause.

| Key | Default | Example | Description |
|---|---|---|---|
| `format` | `"{title} - {artist}"` | `"{artist}: {title}"` | Label format. Placeholders: `{title}`, `{artist}`, `{album}`. Text next to an empty placeholder is dropped, so a track without an artist shows just its title. Write `{{` or `}}` for a literal brace. |
| `max_length` | `40` | `0` | Longest label in characters before it is cut with `…` (the tooltip shows the whole label). `0` means no limit. |
| `hide_when_idle` | `false` | `true` | Hide the module when nothing is playing, instead of showing "No media". |

### `[system]`

CPU, memory and temperature, from `/proc` and `/sys`, on one timer.

| Key | Default | Example | Description |
|---|---|---|---|
| `metrics` | `["cpu", "memory", "temperature"]` | `["cpu", "memory"]` | Metrics to show, in order: any of `"cpu"`, `"memory"`, `"temperature"`. |
| `interval` | `2` | `5` | Seconds between refreshes, at least `1`. One timer serves every metric and every bar. |

### `[system.temperature]`

| Key | Default | Example | Description |
|---|---|---|---|
| `warm` | `70` | `65` | From this many °C the module turns warm (`zb-warm`: colour, icon and tooltip). |
| `hot` | `85` | `90` | From this many °C the module turns hot (`zb-hot`). |
| `sensor` | unset | `"k10temp"` | Pin the sensor instead of finding it: a hwmon chip name (`k10temp`), a thermal zone type (`x86_pkg_temp`) or an absolute path such as `/sys/class/hwmon/hwmon3/temp1_input`. Unset, Zenith looks for `k10temp`, `coretemp` and `zenpower`, then thermal zones. |

### `[todo]`

The todo module has no settings of its own beyond click actions. Todos live in
`$XDG_DATA_HOME/zenith-bar/todos.json` (usually `~/.local/share/zenith-bar/`),
not in your config, so your dotfiles stay clean. Click the count to open the list.

### `[logo]`

| Key | Default | Example | Description |
|---|---|---|---|
| `icon` | `"zb-logo-symbolic"` | `"archlinux-logo"` | A bundled `zb-*-symbolic` icon or any icon name from your icon theme. An empty string hides the logo. |

### `[theme]`

The look. See [theming](theming.md) for what each override changes and for the
`--zb-*` variables your `style.css` can use.

| Key | Default | Example | Description |
|---|---|---|---|
| `name` | `"midnight-bloom"` | `"tokyo-night"` | Built-in theme: `"midnight-bloom"`, `"tokyo-night"`, `"catppuccin-mocha"` or `"gruvbox-dark"`. |
| `accent` | unset | `"#ff79c6"` | Signature colour, overriding the theme's `--zb-accent`. Tinted borders and the glow follow it. |
| `surface_opacity` | unset | `0.9` | Opacity of the bar surface from `0` (clear) to `1` (opaque), overriding `--zb-surface-opacity`. |
| `module_radius` | unset | `6` | Corner radius of module pills in pixels, overriding `--zb-module-radius`. The bar's own corners are `[bar] radius`. |

### Click actions

Every module table (`[clock]`, `[calendar]`, `[media]`, `[system]`, `[todo]`,
`[logo]`) also takes these keys. Each is a shell command, run detached so a slow
command never freezes the bar. A configured command **replaces** the module's
built-in action for that gesture (media play/pause, the calendar and todo
popovers); an empty string disables the built-in action without running anything.

| Key | Default | Example | Description |
|---|---|---|---|
| `on_click` | unset | `"rofi -show drun"` | Run on left click. |
| `on_right_click` | unset | `"wlogout"` | Run on right click. |
| `on_scroll_up` | unset | `"playerctl next"` | Run on scroll up. |
| `on_scroll_down` | unset | `"playerctl previous"` | Run on scroll down. |

## Environment

| Variable | Effect |
|---|---|
| `ZENITH_BAR_CONFIG` | Config file path, if `--config` isn't given. |
| `ZENITH_BAR_STYLE` | Stylesheet path, if `--style` isn't given (default `~/.config/zenith-bar/style.css`). |
| `ZENITH_BAR_LOG` | Log level or filter, e.g. `info` or `debug`. The default, `warn`, keeps your journal quiet. |
| `GSK_RENDERER` | GTK renderer. Zenith uses `cairo` unless this is set; see [performance](performance.md#renderer-comparison). |

## For contributors: keeping the schema in sync

`tests/schema_drift.rs` fails when the committed schema differs from the generated
one. After changing any config type (or its doc comments, which become the
schema's descriptions), regenerate it:

```sh
UPDATE_SCHEMA=1 cargo test --test schema_drift
```
