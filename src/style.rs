use crate::config::BarConfig;

/// Generate the complete GTK4 CSS for the Zenith bar.
///
/// Redesigned to bypass Hyprland blur/shadow bugs:
///   1. Fully transparent root window.
///   2. Outer frame uses the animated RGB gradient (No CSS drop-shadows!).
///   3. Inner panel uses a solid, deep-space opaque color since blur is disabled.
///   4. Font stack utilizes Inter for clean UI, and JetBrainsMono for hardware numbers/icons.
pub fn build_css(bar: &BarConfig) -> String {
    let radius = bar.border_radius;
    let bw = bar.border_width;
    let cycle = bar.rgb_cycle_seconds;
    let inner_radius = radius.saturating_sub(bw);

    format!(
        r#"
/* ── Reset & Transparent Foundation ────────────────────────────── */
window {{
    /* This must be 0 to let the border shape define the bar */
    background-color: rgba(0, 0, 0, 0);
}}

/* ── Outer Frame: The Solid Cosmic Strip ───────────────────────── */
.zenith-border {{
    border-radius: {radius}px;
    padding: {bw}px;
    
    /* High-contrast synthwave gradient */
    background: linear-gradient(
        45deg,
        #ff0055, 
        #7700ff, 
        #00ccff, 
        #00ff99, 
        #7700ff, 
        #ff0055
    );
    background-size: 300% 300%;
    animation: cosmic-flow {cycle}s linear infinite;
    
    /* BUG FIX: Zero box-shadows to prevent Hyprland bounding box artifacts */
    box-shadow: none;
    border: none;
}}

/* ── Inner Surface: Opaque Command Deck ────────────────────────── */
.zenith-inner {{
    /* BUG FIX: A solid, premium deep-space black instead of broken blur */
    background-color: #0d1117; 
    border-radius: {inner_radius}px;
    padding: 2px 18px; 
}}

/* ── Base Typography: Inter ────────────────────────────────────── */
.zenith-module {{
    /* Inter primary for clean text, JetBrains fallback for icons */
    font-family: "Inter", "JetBrainsMono Nerd Font", sans-serif;
    font-size: 14px;
    font-weight: 700;
}}

/* ── Data Typography: JetBrains Mono ───────────────────────────── */
/* Target your Clock and Hardware numbers specifically for tabular spacing */
.zenith-module-center,
.zenith-module-right {{
    font-family: "JetBrainsMono Nerd Font", "Inter", monospace;
    font-weight: 800;
}}

/* Accent Colors */
.zenith-module-left {{
    color: #00ccff; /* Neon Cyan */
}}

.zenith-module-center {{
    color: #ffffff; /* Pure White */
    text-shadow: 0px 0px 8px rgba(255, 255, 255, 0.3); /* Slight text glow */
}}

.zenith-module-right {{
    color: #ff0055; /* Dawn Red */
}}

/* ── Calendar Popover & Button ─────────────────────────────────── */
.zenith-calendar-btn {{
    background: transparent;
    border: none;
    padding: 4px 8px;
    color: #ffffff;
    font-size: 16px;
}}

.zenith-calendar-btn:hover {{
    background: rgba(255, 255, 255, 0.1);
    border-radius: 4px;
}}

.zenith-calendar {{
    background-color: #0d1117;
    border: 1px solid #30363d;
}}

.zenith-calendar-popup {{
    background-color: #0d1117;
    border-radius: 8px;
}}

/* ── Keyframes: The Endless Engine Flow ────────────────────────── */
@keyframes cosmic-flow {{
    0%   {{ background-position: 0% 50%; }}
    50%  {{ background-position: 100% 50%; }}
    100% {{ background-position: 0% 50%; }}
}}
"#,
        radius = radius,
        bw = bw,
        inner_radius = inner_radius,
        cycle = cycle,
    )
}
