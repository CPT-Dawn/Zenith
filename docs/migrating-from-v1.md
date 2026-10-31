# Migrating from Zenith 1.x

Zenith 2 is a rewrite and a deliberate clean break. It doesn't read v1's config, style or todo files, and there is no automatic migration. The quickest path is to start fresh and copy over the few values you care about.

## Start here

```sh
zenith-bar init --preset classic   # the v1 layout in the Tokyo Night palette
```

Then edit `~/.config/zenith-bar/config.toml`, run `zenith-bar check`, and swap `zenith` for `zenith-bar` in your compositor's autostart. Once you're happy, `~/.config/zenith/` can go.

## What moved

| v1 | v2 |
|---|---|
| Binary `zenith` | `zenith-bar` (no more clash with the system monitor of the same name) |
| `~/.config/zenith/config.toml` | `~/.config/zenith-bar/config.toml` |
| `~/.config/zenith/style.css` | `~/.config/zenith-bar/style.css`, now optional: it layers on top of the theme |
| `~/.config/zenith/todos.json` | `~/.local/share/zenith-bar/todos.json`, in a new format with three priorities (high, normal, low) instead of P1–P9. Re-add open todos by hand. |
| `ZENITH_CONFIG`, `ZENITH_STYLE` | `ZENITH_BAR_CONFIG`, `ZENITH_BAR_STYLE` (or `--config`, `--style`) |
| `RUST_LOG` | `ZENITH_BAR_LOG` |
| `GSK_RENDERER` defaulted to `gl` | Defaults to `cairo`; your own `GSK_RENDERER` still wins |
| Layer namespace `zenith` | `zenith-bar`: update Hyprland `layerrule`s |
| `playerctl` required | Not needed: media talks MPRIS over D-Bus directly |
| Nerd Font required | Not needed: icons are built in |
| Restart to apply changes | Config and style reload on save |

## Config keys

v1's keys are not accepted (unknown keys are errors). Rough equivalents:

| v1 | v2 |
|---|---|
| `[bar] monitor = "DP-1"` | `[bar] monitors = ["DP-1"]` (default: a bar on every output) |
| `[bar] height` | `[bar] height` |
| `[bar] gap_horizontal`, `gap_top` | `[bar] margin` (one gap on every side) |
| `[bar] border_radius` | `[bar] radius` |
| `[bar] background` | `[theme] surface_opacity`, or `--zb-surface` in `style.css` |
| `[bar] border_width`, `rgb_cycle_seconds` | `[bar] motion = "ambient"` for the flowing border (it costs wakeups; see [theming](theming.md#motion)) |
| `[modules] clock = true` etc. | List the module in `[layout] left`, `center` or `right`; leave it out to disable it |
| `[modules] clock_format` | `[clock] format` |

## CSS

v1's `.zenith-*` classes are gone. v2 has stable `zb-*` classes and `--zb-*` variables, documented in [theming](theming.md). They are a public contract, so a v2 `style.css` keeps working across releases. Most v1 tweaks become a one-line `[theme]` override (`accent`, `surface_opacity`, `radius`) with no CSS at all.
