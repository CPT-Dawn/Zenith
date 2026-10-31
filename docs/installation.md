# Installation

Zenith runs on any Wayland compositor that implements `wlr-layer-shell`. Hyprland is the most-tested target; Sway is what the benchmark harness and CI run on. GNOME and KDE are not supported.

## Arch Linux (AUR)

```sh
paru -S zenith-bar      # or: yay -S zenith-bar
```

The package builds from source. It installs one binary, `/usr/bin/zenith-bar`.

## From source

You need:

| Dependency | Version | Arch | Debian/Ubuntu | Fedora |
|---|---|---|---|---|
| GTK 4 | 4.22 or newer | `gtk4` | `libgtk-4-dev` | `gtk4-devel` |
| gtk4-layer-shell | 1.3 or newer | `gtk4-layer-shell` | `libgtk4-layer-shell-dev` | `gtk4-layer-shell-devel` |
| Rust toolchain | 1.92 or newer (edition 2024) | `rust` or `rustup` | [rustup](https://rustup.rs) | [rustup](https://rustup.rs) |

GTK 4.22 is recent: if your distribution ships an older GTK, Zenith won't build there yet. The package names above are the usual ones; check your distribution if one is missing.

```sh
git clone https://github.com/CPT-Dawn/Zenith.git
cd Zenith
cargo build --release --locked
sudo install -Dm755 target/release/zenith-bar /usr/local/bin/zenith-bar
```

Run `cargo test` first if you want to check the build on your machine.

### Fonts and icons

- **Text** uses [Inter](https://rsms.me/inter/) when it is installed (`ttf-inter` on Arch), and falls back to your system sans-serif otherwise.
- **Icons** are compiled into the binary as symbolic SVGs, so no Nerd Font is needed. Nerd Font glyphs still work inside format strings if you have one.

## First run

```sh
zenith-bar                         # runs with the default look; no config file needed
zenith-bar init --preset bloom     # or minimal, classic; or no --preset for a commented starter
zenith-bar check                   # validate after editing
```

Config and style live in `~/.config/zenith-bar/` and reload on save. See [configuration](configuration.md) for every key.

Only one Zenith runs per session: launching it again does nothing while one is running.

## Compositor notes

The layer-shell namespace is `zenith-bar`. Use it in any compositor rule that matches layers.

### Hyprland

In `~/.config/hypr/hyprland.conf`:

```ini
exec-once = zenith-bar

# Smoked glass: blur what's behind the bar, but not the transparent gap around it.
layerrule = blur on, match:namespace zenith-bar
layerrule = ignore_alpha 0.3, match:namespace zenith-bar
```

On Hyprland older than 0.53, the layer rules are written the old way:

```ini
layerrule = blur, zenith-bar
layerrule = ignorealpha 0.3, zenith-bar
```

### Sway

In `~/.config/sway/config`:

```text
exec zenith-bar
```

Remove or comment out the `bar { … }` block so swaybar doesn't draw a second bar. Plain Sway doesn't blur layers, so the glass themes show the wallpaper through a tint rather than a blur. For a solid bar, use the `minimal` preset or set `[theme] surface_opacity = 1`.

### niri, river and other wlroots-style compositors

Zenith only needs `wlr-layer-shell`, so it should work anywhere that protocol is implemented. Start it the way your compositor starts any program at login:

- **niri**: `spawn-at-startup "zenith-bar"` in `~/.config/niri/config.kdl`
- **river**: `zenith-bar &` in your `init` script

Without compositor blur, the same advice as for Sway applies: the glass is tinted, not blurred. If something looks wrong on your compositor, please [open an issue](https://github.com/CPT-Dawn/Zenith/issues/new/choose) with its name and version.

## Logs

Zenith logs warnings only, to stderr. For more detail:

```sh
ZENITH_BAR_LOG=debug zenith-bar
```

## Renderer

Zenith draws with GTK's `cairo` renderer, which measured cheapest on every metric while rendering the themes the same as the GPU renderers ([performance](performance.md#renderer-comparison)). To try another one, set `GSK_RENDERER` yourself, e.g. `GSK_RENDERER=ngl zenith-bar`.
