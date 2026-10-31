# Cairo is the default renderer

GTK picks a GPU renderer (Vulkan, then GL) by default. For a bar that redraws a few times a minute that is the wrong trade: the GPU renderers bring a driver context, a renderer thread that keeps waking up while the bar is idle, and a slower first frame, and they buy nothing a strip of text and rounded rectangles needs. Measured under the benchmark harness with Midnight Bloom, Cairo cost the least on every metric, with or without a hardware GPU: on hardware GL it used about 60 % of `ngl`'s proportional memory and first-frame time, and `vulkan` woke up about 125 times a second at idle against Cairo's 1.4; without a GPU, `ngl` and `vulkan` took about four times as long to the first frame. Screenshots show Cairo draws Midnight Bloom the same as they do (`docs/performance.md`). So `zenith-bar` sets `GSK_RENDERER=cairo` at startup unless the user has already set `GSK_RENDERER`.

## Consequences

- The choice is an environment default, not a config key: `GSK_RENDERER=ngl zenith-bar` (or `vulkan`) still works, and it is GTK's own switch, so GTK's documentation applies.
- Cairo draws on the CPU. With `motion = "ambient"` every animation frame is software-rendered; anyone who turns ambient motion on and cares about CPU should measure `ngl` on their machine.
- The comparison was made in headless Sway, not on a real desktop session. If a GTK release or a real-session measurement shows a GPU renderer idling as quietly, revisit this.
