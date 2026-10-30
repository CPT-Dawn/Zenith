# Event-driven, single main loop, no threads or subprocesses

Every module runs on the one glib main loop. Modules are driven in one of three ways:

- **Events:** MPRIS over gio D-Bus, and gio file monitors for hot reload.
- **Boundary-aligned timers:** the clock wakes exactly on the minute or second.
- **Cheap reads:** `/proc` and `/sys` for system stats.

We chose this over tokio, zbus, `sysinfo` or spawning `playerctl`, because GTK already links gio, so these paths add no binary size or memory. Idle wakeups are what the user pays for, and this model keeps them near zero.

## Consequences

- No module may block the main loop. Any slow I/O must go through gio's async APIs.
- Do not "simplify" by spawning helper processes or threads. v1 forked `playerctl` every 700 ms, and that was the largest constant cost.
