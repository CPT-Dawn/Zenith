# Theming

Zenith's look is made of named CSS variables (`--zb-*`) and stable CSS classes (`zb-*`). Both are a public contract: a variable or class listed here keeps its name and meaning, so your `style.css` keeps working across releases.

## Layers

Stylesheets stack from lowest to highest priority. A later layer wins over an earlier one.

1. **Theme**: the built-in theme chosen by `[theme] name`. It only defines `:root { --zb-* }` variables. Only the selected theme is loaded.
2. **Component CSS**: Zenith's own rules. They only *read* variables and never use literal colours, so changing a variable changes everything that uses it.
3. **Overrides**: a `:root` block that Zenith generates from your config: `[bar] height` and `radius`, plus any `[theme]` overrides.
4. **Motion**: a sheet that disables transitions (`motion = "off"` or reduced motion), the ambient animations (`motion = "ambient"`), or nothing (`subtle`).
5. **Your `style.css`**: `~/.config/zenith-bar/style.css`, or the path given by `--style` or `ZENITH_BAR_STYLE`. Loaded at GTK's user priority, it overrides everything above.

## `[theme]`

```toml
[theme]
name = "midnight-bloom"   # built-in theme
accent = "#ff79c6"        # optional: --zb-accent
surface_opacity = 0.9     # optional, 0 to 1: --zb-surface-opacity
radius = 6                # optional, pixels: --zb-module-radius
```

The overrides are variables like any other, so a theme's derived colours follow them. Midnight Bloom tints its borders, module backgrounds and glow from `--zb-accent`, so one `accent` line recolours all of them. `accent` takes a hex colour (`#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`), a colour function (`rgb()`, `rgba()`, `hsl()`, `hsla()`, `hwb()`, `lab()`, `oklab()`, `oklch()`) or a colour name.

Built-in themes: `midnight-bloom` (the default).

## Motion

`[bar] motion` picks the level of animation. The default is `subtle`.

| Value | What moves | Cost |
|---|---|---|
| `off` | Nothing. Every change is instant. | None. |
| `subtle` | Short transitions in response to an event: hover, a track change, a threshold crossed. | None while idle: no frame-clock ticks. |
| `ambient` | Everything in `subtle`, plus the flowing border on `zb-bar` and a pulse on the focal element. | The bar redraws every frame for as long as it is shown: about 4 % of a CPU core and 130+ wakeups/s with software rendering. |

If your system asks for reduced motion (GTK's `gtk-interface-reduced-motion`, usually set by your desktop's accessibility settings), Zenith behaves as `off` whatever `motion` says, and follows the setting live.

Ambient animations live in a separate stylesheet that is only loaded for `ambient`, so `subtle` and `off` cannot run an animation by accident. Please don't add `infinite` animations to your `style.css` either: each one keeps the bar redrawing.

## Focal element

`[bar] focal` names the one module that gets the neon glow (`--zb-glow`): `"clock"` by default, any module in your layout, or `"none"`. Zenith moves the `zb-focal` class to that module. With `ambient` motion its glow pulses towards `--zb-glow-strong`.

## Variables

Every theme defines every variable below.

### Typography

| Variable | Meaning |
|---|---|
| `--zb-font-family` | Font stack for the whole bar. |
| `--zb-font-size` | Base text size. |
| `--zb-font-weight` | Regular text weight. |
| `--zb-font-weight-strong` | Emphasised text weight (the clock). |

### Colours

| Variable | Meaning |
|---|---|
| `--zb-accent` | The signature colour: focal element, logo, accent-tinted surfaces. Overridden by `[theme] accent`. |
| `--zb-accent-blue` | First supporting colour. |
| `--zb-accent-lavender` | Second supporting colour. |
| `--zb-text` | Primary text. |
| `--zb-text-muted` | Secondary text. |
| `--zb-text-dim` | Least important text: placeholders, inactive items. |
| `--zb-ok` | Semantic "good" state. Used only for state. |
| `--zb-warm` | Semantic warning state (`zb-warm`). Used only for state. |
| `--zb-hot` | Semantic critical state (`zb-hot`). Used only for state. |

### Surfaces

| Variable | Meaning |
|---|---|
| `--zb-surface-color` | Bar background colour, without transparency. |
| `--zb-surface-opacity` | Bar background opacity, 0 to 1. Overridden by `[theme] surface_opacity`. |
| `--zb-surface` | The bar background actually painted, usually built from the two above. |
| `--zb-surface-border` | The bar's 1 px border. |
| `--zb-module-bg` | Module pill background. |
| `--zb-module-bg-hover` | Module pill background on hover. |

### Glow

| Variable | Meaning |
|---|---|
| `--zb-glow` | `text-shadow` / `-gtk-icon-shadow` list on the focal element. |
| `--zb-glow-strong` | Peak of the ambient focal pulse. |

### Geometry

| Variable | Meaning |
|---|---|
| `--zb-bar-height` | Bar height. Set from `[bar] height`. |
| `--zb-bar-radius` | Bar corner radius. Set from `[bar] radius`. |
| `--zb-module-radius` | Module pill corner radius. Overridden by `[theme] radius`. |
| `--zb-space-xs` | Extra-small spacing. |
| `--zb-space-s` | Small spacing: bar padding, section margin. |
| `--zb-space-m` | Medium spacing: module padding. |
| `--zb-space-l` | Large spacing. |

### Motion

| Variable | Meaning |
|---|---|
| `--zb-transition-duration` | Length of event-driven transitions. |
| `--zb-transition-easing` | Easing of event-driven transitions. |
| `--zb-ambient-flow-duration` | One cycle of the ambient flowing border. |
| `--zb-ambient-pulse-duration` | One half-cycle of the ambient focal pulse. |

## Classes

### Structure

| Class | On |
|---|---|
| `zb-window` | The bar's layer-shell window. Its background stays transparent. |
| `zb-bar` | The bar surface. |
| `zb-section` | Each of the three sections, together with `zb-left`, `zb-center` or `zb-right`. |
| `zb-left` | The left section. |
| `zb-center` | The center section. |
| `zb-right` | The right section. |
| `zb-module` | Every module. |
| `zb-focal` | The focal element (see above). |
| `zb-popover` | Every popover a module opens, together with `zb-<module>-popover`, e.g. `zb-calendar-popover`. |

### Modules

Each module also carries `zb-<module>`:

| Class | Module |
|---|---|
| `zb-clock` | Clock |
| `zb-calendar` | Calendar |
| `zb-media` | Media player |
| `zb-system` | System metrics |
| `zb-todo` | Todo list |
| `zb-logo` | Logo |

### State

Modules add state classes while a state holds:

| Class | When |
|---|---|
| `zb-warm` | A metric crossed its warning threshold. |
| `zb-hot` | A metric crossed its critical threshold. |
| `zb-playing` | Media is playing. |
| `zb-paused` | Media is paused. |

## Example `style.css`

```css
/* A pinker, more opaque bar with square pills */
:root {
  --zb-accent: #ff79c6;
  --zb-surface-opacity: 0.95;
  --zb-module-radius: 0;
}

/* No pill behind the clock */
.zb-clock {
  background: none;
}
```
