# Changelog

All notable changes to Zenith are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project
uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

v2 is a clean break from v1. Releases before 2.0.0 predate conventional
commits; their notes live on the GitHub Releases page.

## [Unreleased]

### Features

- [**breaking**] Raze v1, lay the zenith-bar config and model seams
- **ui:** One Midnight Bloom bar, a clock and a logo walk into it
- **bench:** Read /proc like a nosy neighbour
- **bench:** Add budgets so regressions get a stern talking-to
- **bench:** Spot the first frame by eavesdropping on wl_surface
- **bench:** Bill Zenith for the helpers it forks
- **bench:** Run the bar in a headless sway terrarium

### Bug Fixes

- **ci:** Strip sway's caps so the container lets it out of bed

### Documentation

- **agents:** Teach the robots where the issues live
- Plant the v2 glossary and ADRs before the rewrite
- **perf:** Weigh v1 honestly; it is not single-digit anything

### Testing

- **aur:** Build the real package and make namcap read it aloud

### Build

- **aur:** Teach the PKGBUILD the zenith-bar name and forget playerctl
- **release:** Let git-cliff write the changelog so humans don't

