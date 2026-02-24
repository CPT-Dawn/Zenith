use crate::config::BarConfig;

/// Generate the complete GTK4 CSS for the Zenith bar.
///
/// The stylesheet accomplishes:
///   1. A fully transparent application window so the compositor can blur
///      whatever is behind it.
///   2. An animated RGB gradient border that continuously rotates via
///      `@keyframes`.
///   3. A translucent dark interior that creates the frosted-glass look once
///      the compositor applies an acrylic / Gaussian blur underneath.
pub fn build_css(bar: &BarConfig) -> String {
    let radius = bar.border_radius;
    let bw = bar.border_width;
    let bg = &bar.background;
    let cycle = bar.rgb_cycle_seconds;

    format!(
        r#"
/* ── Reset & transparent foundation ────────────────────────────── */
window {{
    background-color: rgba(0, 0, 0, 0);
}}

/* ── Outer frame: animated RGB gradient border ─────────────────── */
.zenith-border {{
    border-radius: {radius}px;
    padding: {bw}px;
    /* CSS background gradients for the moving rainbow edge */
    background-image: linear-gradient(
        var(--rgb-angle, 0deg),
        #f38ba8,
        #fab387,
        #f9e2af,
        #a6e3a1,
        #89dceb,
        #b4befe,
        #cba6f7,
        #f38ba8
    );
    background-size: 300% 300%;
    animation: rgb-shift {cycle}s linear infinite;
}}

/* ── Inner surface: translucent dark panel ─────────────────────── */
.zenith-inner {{
    background-color: {bg};
    border-radius: {inner_radius}px;
    padding: 0 12px;
}}

/* ── Module labels ─────────────────────────────────────────────── */
.zenith-module {{
    color: #cdd6f4;
    font-family: "JetBrains Mono", "Fira Code", "Cascadia Code", monospace;
    font-size: 14px;
    font-weight: 600;
}}

.zenith-module-left {{
    color: #89b4fa;
}}

.zenith-module-center {{
    color: #cdd6f4;
}}

.zenith-module-right {{
    color: #a6adc8;
}}

/* ── Keyframes: rotate the gradient angle ──────────────────────── */
@keyframes rgb-shift {{
    0%   {{ background-position: 0% 50%; }}
    50%  {{ background-position: 100% 50%; }}
    100% {{ background-position: 0% 50%; }}
}}
"#,
        radius = radius,
        bw = bw,
        bg = bg,
        inner_radius = radius.saturating_sub(bw),
        cycle = cycle,
    )
}
