# Performance

Zenith's resource claims come from one place: the benchmark harness in `bench/`. This page explains how it measures, how to run it, and the numbers it has produced so far. Each **Performance budget** (see `GLOSSARY.md`) is set from these measurements, never from guesses.

## What is measured

| Metric | Unit | How |
|---|---|---|
| Resident memory | MiB | `VmRSS` from `/proc/<pid>/status` at the end of the window. Shared libraries (GTK, Mesa, GPU driver) count in full. |
| Proportional memory | MiB | `Pss` from `/proc/<pid>/smaps_rollup` at the end of the window: shared pages are split between the processes that map them, so this is closer to what the bar adds to a running desktop. |
| Idle CPU | % of one core | Change in `utime + stime + cutime + cstime` from `/proc/<pid>/stat` over the window, divided by window length. `cutime/cstime` mean helper processes the bar forks and reaps (v1's `playerctl`) are billed to the bar. |
| Wakeups | per second | Change in `voluntary_ctxt_switches + nonvoluntary_ctxt_switches` of every thread in `/proc/<pid>/task/*/status`, divided by window length. Threads are sampled every second and counted one by one, so a pool thread that exits mid-window can't take its history with it. Helper processes' own switches are **not** included. |
| Time to first frame | ms | A separate launch with `WAYLAND_DEBUG=client`. Measured from `spawn()` to the first `wl_surface.commit` that follows a non-null `wl_surface.attach`, read live from the bar's stderr. |

## Method

Every run follows the same steps:

1. Create a temp dir holding a private `XDG_RUNTIME_DIR`, plus empty `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME` and `XDG_STATE_HOME`. Your own config is never read.
2. Start a private `dbus-daemon --session`. This means no media players are visible, and the bar can't hand off to a Zenith already running on your desktop.
3. Start Sway headless with one 1920×1080 output, no swaybar and no Xwayland (`WLR_BACKENDS=headless`, `WLR_RENDERER` from `--renderer`).
4. **First-frame launch.** Start the bar with `WAYLAND_DEBUG=client`, time it to the first frame (15 s timeout), then kill it.
5. **Idle launch.** Start the bar again without protocol logging, so logging can't inflate the idle numbers.
6. Wait the **settle time** (default 10 s), take a sample, wait the **measurement window** (default 30 s), take another sample. With `--screenshot`, `grim` captures the output only after the second sample.
7. With `--smoke`, launch the bar once more and check hot reload on the live compositor: it must draw, redraw after a `style.css` edit, re-anchor to the bottom edge after a config edit, and keep running after an invalid config.
8. Kill everything and remove the temp dir. On failure, the logs are kept and their path is printed.

The settle time and window are fixed defaults. Change them only to investigate something, and say so when you publish numbers.

`--renderer` picks Sway's renderer, and that choice decides what the bar can render with:

- **`pixman`** (the default, and what CI uses): no GPU is needed, but the compositor offers no dmabuf. GTK's GL renderer then falls back to Mesa's llvmpipe software rasteriser, which runs one thread per CPU. This gives comparable numbers on any machine, but they are pessimistic.
- **`gles2`**: uses the machine's GPU render node, so the bar gets hardware GL. This is close to a real desktop session.

Always state which renderer you used.

## Running it

```sh
cargo build --release                      # the Zenith build to measure
cargo bench-bar --bin target/release/zenith-bar \
  [--preset bloom | --config my.toml] [--style my.css] [--renderer gles2] \
  [--smoke] [--screenshot bar.png] [--budget bench/budget.toml]
```

`cargo bench-bar` is an alias for `cargo run --release --manifest-path bench/Cargo.toml --`. The harness is a standalone, dependency-free crate. It works with any Zenith binary: v1's `zenith`, v2's `zenith-bar`, or an installed one found on `PATH`.

- `--config` / `--style` are exported as both `ZENITH_CONFIG`/`ZENITH_STYLE` (v1) and `ZENITH_BAR_CONFIG`/`ZENITH_BAR_STYLE` (v2).
- `--preset NAME` (v2) generates the config with `zenith-bar init --preset NAME`.
- `--env KEY=VALUE` (repeatable) passes extra environment to the bar, e.g. `--env GSK_RENDERER=ngl`.
- `--smoke` adds the hot-reload smoke test (step 7). A failed step exits 1.
- `--screenshot FILE` saves a PNG of the idle output, used to check that a renderer draws the theme correctly. Needs `grim`.
- `--budget FILE` takes `metric = ceiling` lines. The metrics are `rss_mib`, `pss_mib`, `idle_cpu_percent`, `wakeups_per_sec` and `first_frame_ms`. Top-level lines apply to every run; a `[preset]` table adds or overrides ceilings for that preset (with `--preset`). The harness exits 1 if any ceiling is exceeded, and also if a budgeted metric could not be measured.
- If `sway` is not installed, the harness prints why and exits 77 (skipped). `--require-compositor` turns that into a failure; CI uses it.

Exit status: `0` ok, `1` budget exceeded or smoke test failed, `2` error, `77` skipped.

CI (`.github/workflows/bench.yml`) runs the harness's unit tests, then runs every preset (`bloom`, `minimal`, `classic`) with `--smoke --budget bench/budget.toml --require-compositor` on the release build. It runs in an Arch container on a GitHub runner, with `--renderer pixman`, and writes the numbers to the job summary. A budget breach or a failed smoke step fails the job.

## Results

All local numbers below come from one machine, measured on 2026-10-31 with the default settle (10 s) and window (30 s):

**Reference laptop:** Acer Predator PH16-71, Intel Core i7-13700HX (24 threads), 32 GB RAM, NVIDIA GeForce RTX 4070 Max-Q (nvidia-open 615.78.08), on AC, `powersave` governor. Arch Linux, kernel 7.2.11, Sway 1.12 (wlroots 0.20.2), GTK 4.24.1, Mesa 26.2.6.

- **v1** is tag `v1.2.0`, `cargo build --release`, no config file (v1 writes and uses its defaults). `playerctl` was installed; no players were on the private bus.
- **v2** is the `zenith-bar` release build of this branch, run with `--preset`.

Every configuration ran three times, interleaved (each round ran every configuration once) so that drift on the machine hits them all alike. Tables show the median, with the range in brackets.

### Memory depends on the machine's state

Resident and proportional memory are mostly shared mappings: GTK, the fonts, and the NVIDIA GL libraries, which GTK loads even when it draws with cairo. How much of those mappings is resident is decided by the kernel's page cache, not by Zenith. During the main sweep, v2 read about 146 MiB resident. Minutes after it, with nothing rebuilt or upgraded, every v2 preset read 192–203 MiB, and v1 rose by a similar factor (one run each):

| State | Build | Sway | Resident | Proportional |
|---|---|---|---|---|
| main sweep | v1 | `gles2` | 181.4 MiB | 92.9 MiB |
| later | v1 | `gles2` | 321.2 MiB | 169.7 MiB |
| main sweep | v2 bloom | `gles2` | 145.6 MiB | 58.4 MiB |
| later | v2 bloom | `gles2` | 201.9 MiB | 94.4 MiB |
| main sweep | v1 | `pixman` | 259.2 MiB | 197.2 MiB |
| later | v1 | `pixman` | 407.8 MiB | 350.1 MiB |
| main sweep | v2 bloom | `pixman` | 145.9 MiB | 66.8 MiB |
| later | v2 bloom | `pixman` | 202.2 MiB | 131.0 MiB |

In the later state, the bar's smaps showed `Inter.ttc` (13 MiB on disk) mapped four times and fully resident each time, and about 100 MiB of `libnvidia-gpucomp`/`libnvidia-glcore`. Our best guess is that it depends on the size of the page-cache folios those files happen to sit in; that is not proven. Idle CPU, wakeups and time to first frame barely moved between the two states. So: **compare memory only within one sweep**, which is what the tables below do, and read the memory budgets as generous ceilings rather than targets.

The v1 numbers published earlier on this page (262.4 MiB resident and 194 wakeups/s on `gles2`) came from an older revision of the harness and a different machine state; the numbers below replace them.

### Renderer comparison

Midnight Bloom (`--preset bloom`) under each GSK renderer, chosen with `--env GSK_RENDERER=…`.

**Sway `gles2`** (hardware GL, close to a real desktop):

| GSK renderer | Resident | Proportional | Idle CPU | Wakeups | First frame |
|---|---|---|---|---|---|
| `cairo` | 145.6 MiB [145.2–145.7] | 58.2 MiB [58.2–58.3] | 0.000 % [0.000–0.033] | 1.40 /s [1.27–1.63] | 194 ms [182–201] |
| `ngl` | 220.2 MiB [220.1–220.3] | 98.4 MiB [98.4–98.8] | 0.067 % [0.067–0.067] | 1.57 /s [1.10–1.80] | 332 ms [259–465] |
| `vulkan` | 246.2 MiB [246.0–277.1] | 118.7 MiB [118.6–149.7] | 0.400 % [0.333–0.400] | 125.13 /s [120.36–136.89] | 343 ms [322–568] |

**Sway `pixman`** (no dmabuf, so `ngl` falls back to llvmpipe; what CI sees):

| GSK renderer | Resident | Proportional | Idle CPU | Wakeups | First frame |
|---|---|---|---|---|---|
| `cairo` | 145.9 MiB [145.8–145.9] | 66.8 MiB [66.7–66.8] | 0.033 % [0.000–0.067] | 1.37 /s [1.33–1.53] | 199 ms [197–207] |
| `ngl` | 294.2 MiB [293.9–294.3] | 196.8 MiB [196.6–196.9] | 0.100 % [0.067–0.133] | 46.00 /s [32.53–50.06] | 783 ms [779–834] |
| `vulkan` | 312.9 MiB [312.7–313.1] | 207.9 MiB [207.6–207.9] | 0.333 % [0.300–0.367] | 149.65 /s [131.62–154.89] | 827 ms [815–832] |

**Does it render correctly?** Every run above also took a `--screenshot`. Side by side, the three renderers draw the same Midnight Bloom bar: smoked-glass pill, mint accent, and the glow on the clock. ImageMagick's `compare -metric AE` puts the difference between `cairo` and `ngl`/`vulkan` at 419–446 pixels of 2 073 600 (0.02 %), in antialiased text edges. Two `cairo` runs from different rounds already differ by 82 pixels, because the clock, CPU and temperature text change.

**Decision:** `cairo` costs the least on every metric, with or without a GPU, and draws Midnight Bloom the same as the GPU renderers. It is the default: `zenith-bar` sets `GSK_RENDERER=cairo` at startup unless `GSK_RENDERER` is already set, so `GSK_RENDERER=ngl zenith-bar` (or `vulkan`) still works. See ADR-0004. The Vulkan renderer is the one to avoid for an idle bar: about 125 wakeups/s even on hardware.

### v2 by preset (default renderer)

No `--env`, so this is what a user gets. Each idle run was followed by a `--smoke` hot-reload check, which passed every time.

| Preset | Sway | Resident | Proportional | Idle CPU | Wakeups | First frame |
|---|---|---|---|---|---|---|
| `bloom` | `gles2` | 145.6 MiB [145.6–145.9] | 58.4 MiB [58.4–58.5] | 0.000 % [0.000–0.033] | 1.23 /s [1.03–1.37] | 194 ms [191–203] |
| `minimal` | `gles2` | 145.3 MiB [145.3–145.4] | 58.1 MiB [58.1–58.2] | 0.000 % [0.000–0.033] | 0.27 /s [0.23–0.27] | 189 ms [189–194] |
| `classic` | `gles2` | 145.5 MiB [145.5–145.8] | 58.3 MiB [58.2–58.5] | 0.033 % [0.000–0.033] | 1.40 /s [1.37–1.47] | 201 ms [199–203] |
| `bloom` | `pixman` | 145.9 MiB [145.8–145.9] | 66.8 MiB [66.8–66.8] | 0.033 % [0.000–0.033] | 1.40 /s [1.40–1.47] | 203 ms [194–205] |
| `minimal` | `pixman` | 145.6 MiB [145.5–145.7] | 66.5 MiB [66.4–66.5] | 0.000 % [0.000–0.033] | 0.27 /s [0.20–0.37] | 201 ms [193–203] |
| `classic` | `pixman` | 145.7 MiB [145.7–145.8] | 66.7 MiB [66.6–66.7] | 0.033 % [0.033–0.067] | 1.50 /s [1.33–1.67] | 198 ms [190–201] |

Idle CPU is reported in whole clock ticks: one 10 ms tick in a 30 s window reads as 0.033 %.

**GitHub runner** (`ubuntu-latest` host, `archlinux:latest` container, Sway `pixman`, default renderer, one run per preset, CI run 41131289190):

| Preset | Resident | Proportional | Idle CPU | Wakeups | First frame |
|---|---|---|---|---|---|
| `bloom` | 43.2 MiB | 33.4 MiB | 0.033 % | 0.80 /s | 241 ms |
| `minimal` | 42.9 MiB | 32.9 MiB | 0.000 % | 0.30 /s | 242 ms |
| `classic` | 43.3 MiB | 33.4 MiB | 0.000 % | 0.73 /s | 246 ms |

The runner has no NVIDIA libraries and far fewer cores, so its memory is much lower. Compare CI numbers only with other CI numbers.

### v1 and v2 side by side

v1 with its defaults against v2 with `bloom` (the default preset), same machine, same sweep:

| Metric | v1 `gles2` | v2 `gles2` | v1 `pixman` | v2 `pixman` |
|---|---|---|---|---|
| Resident memory | 181.4 MiB [181.3–181.7] | 145.6 MiB | 259.2 MiB [258.9–261.2] | 145.9 MiB |
| Proportional memory | 92.9 MiB [92.9–93.2] | 58.4 MiB | 197.2 MiB [196.9–199.1] | 66.8 MiB |
| Idle CPU | 17.132 % [16.699–17.932] | 0.000 % | 72.594 % [72.593–74.560] | 0.033 % |
| Wakeups | 196.55 /s [191.69–198.06] | 1.23 /s | 51 975.61 /s [51 670.52–52 371.58] | 1.40 /s |
| Time to first frame | 281 ms [280–287] | 194 ms | 1 259 ms [1 241–1 261] | 203 ms |

What the numbers show:

- **v2 idles.** v1 redraws every frame (its always-running CSS animations, ADR-0003) and forks `playerctl` every 700 ms (ADR-0002); in software rendering every frame wakes every llvmpipe thread. v2 is event-driven and draws with cairo, so it wakes about once a second and uses a clock tick or two of CPU per 30 s.
- **The "single-digit megabytes" claim is still wrong.** Most of the resident memory is shared GTK, font and GL driver mappings, which ADR-0001 already expects. Proportional memory is the fairer number, and it is 37–66 % lower than v1's.

### v2 budgets

`bench/budget.toml` holds the **Performance budgets**. Each ceiling is the worst v2 run seen (laptop in either memory state, or the GitHub runner), plus headroom for machine-to-machine noise. CI enforces them with Sway `pixman` on every preset. Locally, the CI command (`--preset … --renderer pixman --smoke --budget bench/budget.toml`) passed for all three presets in the later memory state; the worst memory figures below come from those runs.

| Metric | Budget | Worst v2 run seen | v1 (`gles2` / `pixman`) |
|---|---|---|---|
| Resident memory | 256 MiB | 202.5 MiB | 181.4 / 259.2 MiB (321.2 / 407.8 in the later state) |
| Proportional memory | 160 MiB | 131.1 MiB | 92.9 / 197.2 MiB (169.7 / 350.1 in the later state) |
| Idle CPU | 0.5 % | 0.067 % | 17.132 / 72.594 % |
| Wakeups, `bloom` | 5 /s | 1.63 /s | 196.55 / 51 975.61 /s (v1 defaults) |
| Wakeups, `minimal` | 1.5 /s | 0.43 /s (a spot check after the sweep) | n/a |
| Wakeups, `classic` | 5 /s | 1.67 /s | n/a |
| Time to first frame | 400 ms | 246 ms (runner) | 281 / 1 259 ms |

Tighten a budget when v2 gets leaner. Never loosen one without new measurements on this page.
