# Changelog

All notable changes to Zenith are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project
uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

v2 is a clean break from v1. Releases before 2.0.0 predate conventional
commits; their notes live on the GitHub Releases page.

## [2.2.0] - 2026-11-01

### Features

- **aur:** Ship zenith-bar-bin for people who'd rather not watch LTO

### Bug Fixes

- **aur:** Declare graphene, which GTK quietly dragged in

## [2.1.0] - 2026-11-01

### Features

- **todo:** Put the next todo on the bar, not just a number

### Bug Fixes

- **todo:** Grab the keyboard before the popover, not after it
- **popover:** Give the bar some personal space

## [2.0.0] - 2026-10-31

### Features

- [**breaking**] Raze v1, lay the zenith-bar config and model seams
- **ui:** One Midnight Bloom bar, a clock and a logo walk into it
- **bench:** Read /proc like a nosy neighbour
- **bench:** Add budgets so regressions get a stern talking-to
- **bench:** Spot the first frame by eavesdropping on wl_surface
- **bench:** Bill Zenith for the helpers it forks
- **bench:** Run the bar in a headless sway terrarium
- **system:** Teach the bar to read /proc without breaking a sweat
- **system:** Sniff out the cpu thermometer on amd, intel and the rest
- **system:** Let [system] be configured, within reason
- **system:** One timer, three numbers, zero jitter on the bar
- **calendar:** Teach the date to wait for midnight like a pumpkin
- **calendar:** Pop a themed month open when the date is clicked
- **bar:** A bar on every output, even the fashionably late ones
- **cli:** Add check and init, so typos get caught before they ship
- **config:** Ship a JSON Schema so editors autocomplete for you
- **config:** Let every module table take click-action commands
- **ui:** Run click commands detached so sleep 30 can't nap the bar
- **calendar:** Let the date take click orders and join the starter
- **config:** Add [theme] overrides and a stylesheet stack that knows its place
- **ui:** Motion sheets, so idle bars stop dancing unless asked
- **reload:** Hot-reload config and style on save, sans restart
- **todo:** A list that remembers, and saves without tearing
- **todo:** Put the list on the bar and the keyboard in charge
- **media:** Teach a pure model to read MPRIS minds
- **media:** Wire MPRIS over gio so playerctl can retire
- **reload:** Teach media and todo to follow the config too
- **theme:** Add Tokyo Night, Catppuccin Mocha and Gruvbox Dark
- **preset:** Ship Bloom, Minimal and Classic with init --preset
- **bench:** Budget per preset, smoke-test reloads, count every wakeup
- **bench:** --screenshot the idle bar, so renderers can prove their art
- [**breaking**] Zenith v2, the lightest beautiful status bar for Wayland (#17)

### Bug Fixes

- **ci:** Strip sway's caps so the container lets it out of bed
- **config:** Report every bad value, not just the one serde tripped on
- **theme:** [**breaking**] Rename [theme] radius to module_radius, one radius each
- **media:** Tick once per pixel of progress, not once per 1/200th
- **reload:** Keep watching when the config dir shows up late
- **reload:** Read the saved config with gio async, per ADR-0002

### Performance

- **ui:** Draw with cairo unless GSK_RENDERER says otherwise
- **bench:** Set budgets from measurements, not vibes

### Refactoring

- **config:** Drop resolve's preset layer that nobody ever wore
- **ui:** Call an output an output, as the glossary insists
- **layout:** Give Section a type instead of three loose strings
- **modules:** Say the strftime check and weak-upgrade dance once
- **todo:** Let Priority name, style and cycle itself
- **reload:** Rename File to WatchedFile so gio::File can breathe

### Documentation

- **agents:** Teach the robots where the issues live
- Plant the v2 glossary and ADRs before the rewrite
- **perf:** Weigh v1 honestly; it is not single-digit anything
- **config:** Teach the schema and starter config about click actions
- **theming:** Map every --zb-* knob and zb-* hook, drift-tested
- **changelog:** Start the paper trail for v2
- **perf:** Publish the renderer bake-off and v1-vs-v2 budgets
- **config:** Document every key, and test it so the docs can't fib
- Add install guide and a v1 migration note for the clean break
- **repo:** Add contributing guide, issue forms and a PR template
- **readme:** Rewrite for v2 with a hero shot and measured numbers
- **readme:** Hang a gallery, and let the blur do the talking
- **hyprland:** Speak Lua, since hyprland.lua is where users live now

### Testing

- **aur:** Build the real package and make namcap read it aloud
- **release:** Render notes per tag so reruns don't time-travel
- **bench:** Plug and unplug outputs, and catch a late first config

### Build

- **aur:** Teach the PKGBUILD the zenith-bar name and forget playerctl
- **release:** Let git-cliff write the changelog so humans don't
- **aur:** Warn 1.x upgraders about the rename and declare pango

