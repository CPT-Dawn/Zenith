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

**Cockpit**:
The module that gathers what you keep an eye on: the runner fleet and every **Feed**, as **Glance** chips on the bar and cards in its popover. Replaces the `runners` module, which stays one release as a deprecated alias.
_Avoid_: Dashboard, panel, hub

**Feed**:
A user-configured command whose stdout is one JSON document (contract v1) that the **Cockpit** renders. Zenith knows nothing about what a feed watches.
_Avoid_: Plugin, provider, integration

**Glance**:
The short items a **Feed** offers for the bar ("Codex 87%"), shown as chips after the runner dots; at most six on a bar.
_Avoid_: Badge, summary

**Tone**:
How a feed value should look: `ok`, `busy`, `warn`, `critical` or `muted`. Each maps to a theme token; an unknown tone reads as `muted`.
_Avoid_: Severity, status, colour

**Runner slot**:
One long-lived place where a GitHub Actions self-hosted runner runs. An ephemeral runner registers under a new name for every job (`ci-1-20261116-221235`); every registration whose name differs only by such a trailing date or time belongs to the same slot. The **Cockpit** shows slots, not registrations.
_Avoid_: Runner (for the slot), agent, worker

**Stale registration**:
An offline runner registration left behind by a slot that has a newer one, e.g. after its container was killed before it could deregister.
_Avoid_: Ghost runner, dead runner

## Relationships

- A **Bar** is shown on exactly one **Output**; an **Output** has at most one **Bar**.
- A **Bar** has three **Sections**; a **Module** appears in at most one **Section**.
- A **Preset** names exactly one **Theme**; a **Theme** can be used by many **Presets**.
- A **Theme** has at most one **Focal element**.
- A **Cockpit** has any number of **Feeds**; each **Feed** has **Glance** items and sections, each coloured by a **Tone**.
- Every **Bar** shows the same **Modules**, which share a single data source per **Module** regardless of how many **Bars** exist.
