# Zenith

Zenith is a lightweight, aesthetics-first status bar for Wayland compositors that implement layer-shell. Its promise is the lowest resource cost of any rich, beautiful bar.

## Language

**Bar**:
One status-bar surface shown on one output. A single Zenith process can show many bars.
_Avoid_: Panel, window

**Output**:
A physical or virtual display that a bar can be placed on, identified by its connector name (e.g. `DP-1`).
_Avoid_: Screen, display, monitor (in prose; `monitors` is acceptable as a config key)

**Module**:
A self-contained unit of information or interaction placed on a bar (clock, media, system, todo, logo, …).
_Avoid_: Widget, block, applet, component

**Section**:
One of the three ordered slots of a bar — `left`, `center`, `right` — each holding a list of modules.
_Avoid_: Cluster, zone, region

**Theme**:
The look of a bar: colours, typography, radii and glow, expressed as named design tokens.
_Avoid_: Style, skin, colorscheme

**Preset**:
A complete starter configuration — layout, geometry, motion and a theme — that a user can begin from.
_Avoid_: Template, profile, theme (a preset *contains* a theme)

**Focal element**:
The single module on a bar that receives the theme's neon glow treatment.
_Avoid_: Highlight, hero

**Motion**:
The user-chosen level of animation on the bar: `off`, `subtle` (transitions only in response to events) or `ambient` (continuous decorative animation).
_Avoid_: Animations setting, effects

**Ambient animation**:
Any animation that runs without a triggering event (e.g. a flowing border). Allowed only when motion is `ambient`.
_Avoid_: Idle animation, background effect

**Performance budget**:
A published, measured ceiling on Zenith's resource use (idle CPU, wakeups, memory, startup time) that every release must meet.
_Avoid_: Target, goal

## Relationships

- A **Bar** is shown on exactly one **Output**; an **Output** has at most one **Bar**.
- A **Bar** has three **Sections**; a **Module** appears in at most one **Section**.
- A **Preset** names exactly one **Theme**; a **Theme** can be used by many **Presets**.
- A **Theme** has at most one **Focal element**.
- Every **Bar** shows the same **Modules**, which share a single data source per **Module** regardless of how many **Bars** exist.
