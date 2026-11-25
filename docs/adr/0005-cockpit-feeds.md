# The cockpit renders feeds from user commands, through a versioned JSON contract

The `runners` module grew into the `cockpit`: the runner fleet plus anything the user wants watched (AI usage limits, job queues, leases). Zenith ships to everyone on GitHub and the AUR, so it must not learn about any of those services. Instead the user configures **feed** commands; each prints one JSON document (contract version 1) describing **glance** chips for the bar and sections (meter and text rows) for the cockpit popover, and Zenith renders whatever it says. Personal logic and credentials stay in the user's command.

## Decisions

- **Contract v1.** `version` must be `1`, anything else is refused with a clear error. Unknown fields and row kinds are ignored and an unknown **tone** reads as `muted`, so feeds can grow without breaking older bars. A required field that is missing or of the wrong type rejects the document. `generated` is checked but not kept, so two runs that say the same thing compare equal and nothing redraws.
- **Runtime.** This is a deliberate, bounded exception to ADR 0002's "no subprocesses": the user asked for the command, as with `token_command` and click actions, and the bar itself never forks a helper of its own. Each feed runs as an async `gio::Subprocess` (`sh -c`) on the main loop, in its own process group so a timeout kills its children too. One loop per feed: run, publish, sleep `interval` (at least 5 s), so a feed never runs twice at once. A timeout, non-zero exit or bad document keeps the last good document and marks the feed failed (a muted `!` chip with the reason).
- **No ambient work (ADR 0003).** Countdowns, ages and job times tick once a second only while the cockpit popover is open. Nothing of the popover exists while it is closed. The bar redraws chips only when the parsed chips change.
- **The runner fleet is one card.** `RunnersSource` is unchanged and owned by the cockpit source; with no `repos` it never asks GitHub and never runs `token_command`, and the cockpit leaves its glance and card out.
- **Config.** `[cockpit]` takes every `[runners]` key, unchanged, plus `[[cockpit.feeds]]` (`command`, `interval`, `timeout`).
- **Deprecation, one release.** The `runners` module stays as an alias showing the runner fleet alone (its own source, its `zb-runners` classes), and `[runners]` is read as a cockpit without feeds. `zenith-bar check` and the log warn about both. Writing both tables, or both modules in `[layout]`, is an error: they would show (and poll) the same fleet twice.
- **Classes.** New parts are `zb-cockpit-*`; tones are `zb-tone-*` (ok = accent, busy = the running-runner colour, warn = `--zb-warm`, critical = `--zb-hot`, muted = `--zb-text-muted`). The runner card keeps its `zb-runners-*` classes in both modules, so existing `style.css` rules keep working.

## Consequences

- A slow feed costs the user's command, not the bar: the main loop only waits on pipes. A cold `cockpit-feed` taking ~9 s is fine under the default 30 s timeout.
- Feed authors get one documented format (docs/configuration.md, "Cockpit feeds"); breaking changes need `version: 2` and a bar that reads it.
- In the release after next, `ModuleId::Runners`, `[runners]` and `zb-runners` (the module class) go.
