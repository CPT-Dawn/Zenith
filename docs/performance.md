# Performance

Zenith's resource claims come from one place: the benchmark harness in `bench/`. This page explains how it measures, how to run it, and the numbers it has produced so far. Each **Performance budget** (see `GLOSSARY.md`) is set from these measurements, never from guesses.

## What is measured

| Metric | Unit | How |
|---|---|---|
| Resident memory | MiB | `VmRSS` from `/proc/<pid>/status` at the end of the window. Shared libraries (GTK, Mesa, GPU driver) count in full. |
| Idle CPU | % of one core | Change in `utime + stime + cutime + cstime` from `/proc/<pid>/stat` over the window, divided by window length. `cutime/cstime` mean helper processes the bar forks and reaps (v1's `playerctl`) are billed to the bar. |
| Wakeups | per second | Change in `voluntary_ctxt_switches + nonvoluntary_ctxt_switches`, summed over every thread in `/proc/<pid>/task/*/status`, divided by window length. Helper processes' own switches are **not** included. |
| Time to first frame | ms | A separate launch with `WAYLAND_DEBUG=client`. Measured from `spawn()` to the first `wl_surface.commit` that follows a non-null `wl_surface.attach`, read live from the bar's stderr. |

## Method

Every run follows the same steps:

1. Create a temp dir holding a private `XDG_RUNTIME_DIR`, plus empty `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME` and `XDG_STATE_HOME`. Your own config is never read.
2. Start a private `dbus-daemon --session`. This means no media players are visible, and the bar can't hand off to a Zenith already running on your desktop.
3. Start Sway headless with one 1920×1080 output, no swaybar and no Xwayland (`WLR_BACKENDS=headless`, `WLR_RENDERER` from `--renderer`).
4. **First-frame launch.** Start the bar with `WAYLAND_DEBUG=client`, time it to the first frame (15 s timeout), then kill it.
5. **Idle launch.** Start the bar again without protocol logging, so logging can't inflate the idle numbers.
6. Wait the **settle time** (default 10 s), take a sample, wait the **measurement window** (default 30 s), take another sample.
7. Kill everything and remove the temp dir. On failure, the logs are kept and their path is printed.

The settle time and window are fixed defaults. Change them only to investigate something, and say so when you publish numbers.

`--renderer` picks Sway's renderer, and that choice decides what the bar can render with:

- **`pixman`** (the default, and what CI uses): no GPU is needed, but the compositor offers no dmabuf. GTK's GL renderer then falls back to Mesa's llvmpipe software rasteriser, which runs one thread per CPU. This gives comparable numbers on any machine, but they are pessimistic.
- **`gles2`**: uses the machine's GPU render node, so the bar gets hardware GL. This is close to a real desktop session.

Always state which renderer you used.

## Running it

```sh
cargo build --release                      # the Zenith build to measure
cargo bench-bar --bin target/release/zenith-bar \
  [--config my.toml] [--style my.css] [--renderer gles2] [--budget budget.toml]
```

`cargo bench-bar` is an alias for `cargo run --release --manifest-path bench/Cargo.toml --`. The harness is a standalone, dependency-free crate. It works with any Zenith binary: v1's `zenith`, v2's `zenith-bar`, or an installed one found on `PATH`.

- `--config` / `--style` are exported as both `ZENITH_CONFIG`/`ZENITH_STYLE` (v1) and `ZENITH_BAR_CONFIG`/`ZENITH_BAR_STYLE` (v2).
- `--env KEY=VALUE` (repeatable) passes extra environment to the bar, e.g. `--env GSK_RENDERER=cairo`.
- `--budget FILE` takes a flat list of `metric = ceiling` lines. The metrics are `rss_mib`, `idle_cpu_percent`, `wakeups_per_sec` and `first_frame_ms`. The harness exits 1 if any is exceeded, and also if a budgeted metric could not be measured.
- If `sway` is not installed, the harness prints why and exits 77 (skipped). `--require-compositor` turns that into a failure; CI uses it.

Exit status: `0` ok, `1` budget exceeded, `2` error, `77` skipped.

CI (`.github/workflows/bench.yml`) runs the harness's unit tests and benchmarks the release build. It runs on an Arch container on a GitHub runner, with `--renderer pixman`, and writes the numbers to the job summary.

## Results

### v1 baseline (v1.2.0)

Source identical to tag `v1.2.0` (built at `04b45a1`), `cargo build --release`, no config file (v1 writes and uses its defaults), default settle 10 s and window 30 s. `playerctl` was installed, and no players were on the private bus.

**Hardware (local):** Acer Predator PH16-71, Intel Core i7-13700HX (24 threads), 32 GB RAM, NVIDIA GeForce RTX 4070 Max-Q (nvidia-open 615.78.08), on AC, `powersave` governor. Arch Linux, kernel 7.2.11, Sway 1.12 (wlroots 0.20.2), GTK 4.24.1, Mesa 26.2.6. Measured 2026-10-31.

Three runs per renderer. The table shows the median, with the range in brackets.

| Renderer | Resident memory | Idle CPU | Wakeups | Time to first frame |
|---|---|---|---|---|
| `gles2` (hardware GL) | 262.4 MiB [262.3–262.5] | 13.1 % [9.0–17.1] | 194 /s [171–225] | 233 ms [214–296] |
| `pixman` (llvmpipe) | 339.5 MiB [339.2–339.5] | 71.9 % [71.3–72.1] | 52 875 /s [52 867–52 955] | 1 295 ms [1 224–1 384] |

**GitHub runner** (`ubuntu-latest` host, `archlinux:latest` container, `pixman`, one run): 160.9 MiB, 40.3 % CPU, 6 237 wakeups/s, 1 053 ms to first frame. The runner has far fewer cores, so llvmpipe starts fewer threads, which most likely accounts for the lower memory and wakeups. Compare CI numbers only with other CI numbers.

What the numbers show:

- **The "single-digit megabytes" claim is wrong by two orders of magnitude.** Most of the resident memory is shared GTK and GL driver mappings, which ADR-0001 already expects.
- **v1 never idles.** Its always-running CSS animations redraw every frame (ADR-0003), and it forks `playerctl` every 700 ms (ADR-0002). In software rendering, every frame wakes every llvmpipe thread, which is why wakeups run into the tens of thousands per second.
- **The `gles2` CPU spread (9–17 %) is wide.** The likely cause is the `playerctl` forks landing unevenly in the window. v2 has no forks, so its spread should be much tighter.

### v2

Not measured yet. v2 numbers and the first published budgets will be recorded here, using the same method.
