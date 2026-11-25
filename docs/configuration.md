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
`logo`, `todo`, `clock`, `calendar`, `media`, `system`, `clipboard`, `cockpit` (and
the deprecated `runners`). A module may appear only
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
not in your config, so your dotfiles stay clean. A symlinked `todos.json` stays a
symlink. A file Zenith can't parse is moved aside to `todos.json.corrupt` (then
`.corrupt.1`, ...) before anything is saved; a file it can't read at all is left
untouched, and changes made in that run are not saved.

The bar shows the most urgent open todo (high before normal before low, oldest
first) and how many more are open, e.g. `Ship the release +2`. Text longer than
32 characters is cut with `…`; hover for the whole todo. With nothing open, only
the icon is left. Click it to open the list.

### `[clipboard]`

Recent clipboard history from the [Hoarder](https://github.com/CPT-Dawn/Hoarder)
clipboard daemon, over its socket. Not in the default layout: add `"clipboard"`
to a section to use it. It needs Hoarder installed (`paru -S hoarder-bin`) and its
Daemon running (`systemctl --user enable --now hoarder`).

The icon shows whether Hoarder is recording, paused (hover to see until when) or
offline. Left click opens the 10 most recent Clips, with thumbnails for images;
click one to make it the clipboard again. Right click pauses recording, or
resumes it. With no Hoarder running the module just shows the offline icon and
quietly tries again, every few seconds at first and then twice a minute, so you
can start Hoarder before or after the bar.

| Key | Default | Example | Description |
|---|---|---|---|
| `search_command` | `"hoarder picker"` | `"~/bin/clips"` | Shell command run by the popover's "Search all…" row, which closes the popover first. |

### `[cockpit]`

The cockpit: your GitHub Actions **self-hosted** runners and whatever your own
**feed** commands report, in one module. Not in the default layout: add
`"cockpit"` to a section, then list repos, feeds, or both.

On the bar: an icon, the runner dots and queued count (when `repos` is set), then
each feed's **glance** chips (`Codex 87%`), coloured by their **tone**, at most 6.
Click for the cockpit: one card per feed section, in feed order, then the runner
card. It scrolls when it is taller than the output.

**Runners.** The bar shows one dot per runner **slot**: filled while it runs a job, hollow
while idle, red when offline, a red ring when an `expected` slot isn't registered.
`+2` counts jobs waiting for a runner. Ephemeral runners register under a new name
for every job (`ci-1-20261116-221235`); registrations that differ only by such a
trailing date or time are one slot, and a slot's leftover offline registrations
count as **stale** instead of as extra runners. Hover for the totals. The runner
card lists every slot with its labels and the job it runs, with a live elapsed
time; click a job to open it, or "Actions ↗" for the repo's Actions page. GitHub's
own hosted runners (`ubuntu-latest`, …) aren't listed by the API, so they don't
show. With no `repos`, nothing asks GitHub and `token_command` never runs.

Each poll makes, per repo, one request for the runners, two for the active runs,
and one per active run for its jobs (at most 20 in-progress and 20 queued runs; long listings are read page by page). Only jobs that ask for a `self-hosted` runner count as queued. Replies are cached with their `ETag`, so an
unchanged reply costs nothing against GitHub's rate limit (5,000 requests an hour).

**Feeds** are listed as `[[cockpit.feeds]]` entries; see [Cockpit feeds](#cockpit-feeds)
for their keys and for the JSON a feed prints.

| Key | Default | Example | Description |
|---|---|---|---|
| `repos` | `[]` | `["CPT-Dawn/PeekaBus"]` | Repositories whose runners are shown, as `"owner/name"`. |
| `expected` | `0` | `7` | How many runner slots you run, across all repos. Fewer registered slots show as missing. `0` turns the check off. |
| `interval` | `30` | `60` | Seconds between polls, at least 10. |
| `token_command` | `"gh auth token"` | `"pass github/token"` | Shell command printing a GitHub token that can read the repos' runners (repo admin, or a classic token with `repo`). Run once, and again after GitHub refuses the token, so a keyring that unlocks after login is picked up. |
| `api_url` | `"https://api.github.com"` | `"https://ghe.example.com/api/v3"` | GitHub API base URL; change it for GitHub Enterprise Server. |
| `feeds` | `[]` | `[{ command = "my-feed" }]` | Feed commands, usually written as `[[cockpit.feeds]]` tables. See [Cockpit feeds](#cockpit-feeds). |

### `[runners]`

**Deprecated**, and gone in the next release: the old name of `[cockpit]`, with
the same keys except `feeds`. A `"runners"` entry in `[layout]` still shows the
runner fleet alone, as before. `zenith-bar check` warns about both; rename them to
`cockpit` (see [migrating](migrating-from-v1.md#from-24-runners-becomes-cockpit)).
Writing both `[runners]` and `[cockpit]`, or both modules in `[layout]`, is an error.

| Key | Default | Example | Description |
|---|---|---|---|
| `repos` | as in `[cockpit]` | `["CPT-Dawn/PeekaBus"]` | Repositories whose runners are shown, as `"owner/name"`. |
| `expected` | as in `[cockpit]` | `7` | How many runner slots you run, across all repos. Fewer registered slots show as missing. `0` turns the check off. |
| `interval` | as in `[cockpit]` | `60` | Seconds between polls, at least 10. |
| `token_command` | as in `[cockpit]` | `"pass github/token"` | Shell command printing a GitHub token that can read the repos' runners (repo admin, or a classic token with `repo`). Run once, and again after GitHub refuses the token, so a keyring that unlocks after login is picked up. |
| `api_url` | as in `[cockpit]` | `"https://ghe.example.com/api/v3"` | GitHub API base URL; change it for GitHub Enterprise Server. |

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
`[logo]`, `[clipboard]`, `[cockpit]`) also takes these keys. Each is a shell command, run
detached so a slow command never freezes the bar. A configured command
**replaces** the module's built-in action for that gesture (media play/pause,
the calendar, todo, clipboard and cockpit popovers, clipboard pause); an empty string
disables the built-in action without running anything.

| Key | Default | Example | Description |
|---|---|---|---|
| `on_click` | unset | `"rofi -show drun"` | Run on left click. |
| `on_right_click` | unset | `"wlogout"` | Run on right click. |
| `on_scroll_up` | unset | `"playerctl next"` | Run on scroll up. |
| `on_scroll_down` | unset | `"playerctl previous"` | Run on scroll down. |

## Cockpit feeds

A feed is any command that prints one JSON document. Zenith runs it, renders what
it describes, and knows nothing else about it: your personal data stays in your
command.

```toml
[[cockpit.feeds]]
command = "my-feed"   # run with sh -c; stdout is one JSON document
interval = 15
timeout = 30
```

| Key | Default | Description |
|---|---|---|
| command | (required) | Shell command, run with `sh -c` in a process group of its own, off the main loop. Its stdout is the feed. |
| interval | 15 | Seconds from the end of one run to the start of the next; at least 5. A feed never runs twice at once. |
| timeout | 30 | Seconds a run may take; then the command and its children are killed. At least 1. |

A run fails when it times out, exits non-zero, or prints something that isn't a
valid feed. The cockpit then keeps showing the last good answer, and the bar adds
a muted `!` chip whose tooltip says why. The reason is logged too.

### The feed format (version 1)

```json
{
  "version": 1,
  "generated": "2026-11-25T15:29:09Z",
  "glance": [
    { "id": "codex", "label": "Codex", "text": "87%", "tone": "critical", "tooltip": "Codex: week 87%" }
  ],
  "sections": [
    {
      "id": "codex", "title": "Codex", "note": "plus",
      "rows": [
        { "kind": "meter", "label": "week", "value": 0.87, "text": "87%", "tone": "critical",
          "until": "2026-11-28T02:00:56Z" },
        { "kind": "text", "label": "status", "text": "all good", "tone": "ok",
          "url": "https://example.com/status" }
      ]
    }
  ]
}
```

- `version` must be `1`; any other version is refused with a clear error.
- `generated` is an RFC 3339 time.
- `glance`: the bar's chips, `label` then `text`, coloured by `tone`; `tooltip` is optional.
- `sections`: one card each, in order. `note` shows muted in its header. `error`
  says the section's own source failed; with no rows the card shows just the error.
  `stale_since` (RFC 3339) says the rows are an older answer; the header shows `⚠ 10m old`.
- A `meter` row: `label`, a bar filled to `value` (0 to 1), `text`, and with
  `until` (RFC 3339) a live countdown such as `3h36m` or `2d10h`. Countdowns tick
  only while the cockpit is open.
- A `text` row: `label` on the left, `text` on the right coloured by the optional
  `tone`; with `url`, a click opens it.
- **Tones**: `ok`, `busy`, `warn`, `critical`, `muted`. An unknown tone shows as `muted`.
- Unknown fields and unknown row kinds are ignored, so the format can grow
  without breaking older bars.

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

[Back to the README](../README.md)
