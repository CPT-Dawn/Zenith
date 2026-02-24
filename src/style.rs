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
    background-color: #211f49; 
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

/* ── Arch Logo ─────────────────────────────────────────────────── */
.zenith-logo {{
    font-family: "JetBrainsMono Nerd Font", monospace;
    font-size: 18px;
    color: #00ccff;
    text-shadow: 0px 0px 10px rgba(0, 204, 255, 0.5);
    padding: 0 2px;
}}

/* ── Calendar: Clickable Date Button ───────────────────────────── */
.zenith-calendar-btn {{
    background: transparent;
    border: none;
    box-shadow: none;
    padding: 4px 10px;
    color: #ffffff;
    font-size: 14px;
    font-weight: 800;
    min-height: 0;
    min-width: 0;
}}

.zenith-calendar-btn:hover {{
    background: rgba(255, 255, 255, 0.08);
    border-radius: 6px;
}}

.zenith-calendar-btn:active {{
    background: rgba(255, 255, 255, 0.14);
}}

/* ── Calendar: Slide-down Popover ──────────────────────────────── */
.zenith-calendar-popup {{
    background-color: #0d1117;
    border: 1px solid #30363d;
    border-radius: 12px;
    padding: 8px;
}}

/* GTK popover inner contents wrapper */
.zenith-calendar-popup > contents {{
    background-color: #0d1117;
    border-radius: 12px;
    padding: 4px;
}}

.zenith-calendar {{
    background-color: transparent;
    color: #cdd6f4;
    font-family: "Inter", "JetBrainsMono Nerd Font", sans-serif;
    font-size: 13px;
}}

/* Calendar header (month/year navigation) */
.zenith-calendar > header {{
    color: #ffffff;
    font-weight: 700;
}}

.zenith-calendar > header > button {{
    color: #00ccff;
    background: transparent;
    border: none;
}}

.zenith-calendar > header > button:hover {{
    background: rgba(0, 204, 255, 0.15);
    border-radius: 6px;
}}

/* Day cells */
.zenith-calendar :selected {{
    background-color: #7700ff;
    color: #ffffff;
    border-radius: 50%;
}}

.zenith-calendar .day-number:hover {{
    background: rgba(255, 255, 255, 0.08);
    border-radius: 50%;
}}

/* ═══════════════════════════════════════════════════════════════════
   TODO MODULE: Task Pulse
   ═══════════════════════════════════════════════════════════════════ */

/* ── Bar Button: Base ──────────────────────────────────────────── */
.zenith-todo-btn {{
    background: transparent;
    border: none;
    box-shadow: none;
    padding: 4px 12px;
    min-height: 0;
    min-width: 0;
    font-family: "JetBrainsMono Nerd Font", "Inter", monospace;
    font-size: 13px;
    font-weight: 700;
    transition: all 200ms ease;
}}

/* Empty state: pulsing + icon */
.zenith-todo-btn-empty {{
    color: #00ccff;
    text-shadow: 0px 0px 12px rgba(0, 204, 255, 0.5);
    animation: todo-pulse 2s ease-in-out infinite;
}}

/* Active state: has tasks, calm glow */
.zenith-todo-btn-active {{
    color: #00ff99;
    text-shadow: 0px 0px 6px rgba(0, 255, 153, 0.3);
}}

/* Urgent state: many pending tasks */
.zenith-todo-btn-urgent {{
    color: #ff5555;
    text-shadow: 0px 0px 8px rgba(255, 85, 85, 0.4);
    animation: todo-urgent 1.5s ease-in-out infinite;
}}

.zenith-todo-btn:hover {{
    background: rgba(255, 255, 255, 0.08);
    border-radius: 6px;
}}

.zenith-todo-btn:active {{
    background: rgba(255, 255, 255, 0.14);
}}

/* ── Popover Shell ─────────────────────────────────────────────── */
.zenith-todo-popup {{
    background-color: #0d1117;
    border: 1px solid #30363d;
    border-radius: 14px;
    padding: 0;
}}

.zenith-todo-popup > contents {{
    background-color: #0d1117;
    border-radius: 14px;
    padding: 0;
}}

/* ── Container ─────────────────────────────────────────────────── */
.zenith-todo-container {{
    padding: 0;
}}

/* ── Header ────────────────────────────────────────────────────── */
.zenith-todo-header {{
    padding: 12px 14px 6px 14px;
}}

.zenith-todo-title {{
    font-family: "Inter", "JetBrainsMono Nerd Font", sans-serif;
    font-size: 14px;
    font-weight: 800;
    color: #ffffff;
    text-shadow: 0px 0px 8px rgba(255, 255, 255, 0.15);
}}

.zenith-todo-progress {{
    font-family: "JetBrainsMono Nerd Font", monospace;
    font-size: 11px;
    font-weight: 700;
    color: #8b949e;
    background: rgba(255, 255, 255, 0.06);
    border-radius: 8px;
    padding: 2px 8px;
}}

/* ── Progress Bar ──────────────────────────────────────────────── */
.zenith-todo-progress-track {{
    margin: 4px 14px 2px 14px;
    min-height: 3px;
    background: rgba(255, 255, 255, 0.06);
    border-radius: 2px;
}}

.zenith-todo-progress-fill {{
    min-height: 3px;
    border-radius: 2px;
    transition: all 300ms ease;
}}

.zenith-todo-fill-low {{
    background: linear-gradient(90deg, #ff0055, #ff5555);
}}

.zenith-todo-fill-mid {{
    background: linear-gradient(90deg, #ff8800, #ffcc00);
}}

.zenith-todo-fill-high {{
    background: linear-gradient(90deg, #00ff99, #00ccff);
}}

/* ── Separator ─────────────────────────────────────────────────── */
.zenith-todo-sep {{
    margin: 4px 14px;
    background: #21262d;
    min-height: 1px;
}}

/* ── Scrollable List ───────────────────────────────────────────── */
.zenith-todo-scroll {{
    background: transparent;
}}

.zenith-todo-list {{
    padding: 4px 8px;
}}

/* ── Individual Row ────────────────────────────────────────────── */
.zenith-todo-row {{
    padding: 6px 6px;
    border-radius: 8px;
    transition: background 150ms ease;
}}

.zenith-todo-row:hover {{
    background: rgba(255, 255, 255, 0.04);
}}

.zenith-todo-row-done {{
    opacity: 0.5;
}}

/* ── Priority Accent Strip ─────────────────────────────────────── */
.zenith-todo-accent {{
    border-radius: 2px;
    min-width: 3px;
    margin: 2px 0;
}}

.zenith-todo-prio-high {{
    background: #ff0055;
    box-shadow: 0 0 4px rgba(255, 0, 85, 0.4);
}}

.zenith-todo-prio-mid {{
    background: #ffcc00;
}}

.zenith-todo-prio-low {{
    background: #00ccff;
}}

.zenith-todo-prio-none {{
    background: #30363d;
}}

/* ── Checkbox ──────────────────────────────────────────────────── */
.zenith-todo-check {{
    min-width: 16px;
    min-height: 16px;
}}

/* ── Task Text ─────────────────────────────────────────────────── */
.zenith-todo-text {{
    font-family: "Inter", sans-serif;
    font-size: 13px;
    font-weight: 500;
    color: #cdd6f4;
}}

.zenith-todo-text-done {{
    text-decoration: line-through;
    color: #6e7681;
}}

/* ── Priority Badge ────────────────────────────────────────────── */
.zenith-todo-badge {{
    font-family: "JetBrainsMono Nerd Font", monospace;
    font-size: 10px;
    font-weight: 800;
    padding: 1px 6px;
    border-radius: 6px;
    margin: 0 2px;
}}

.zenith-todo-badge-high {{
    color: #ff0055;
    background: rgba(255, 0, 85, 0.15);
}}

.zenith-todo-badge-mid {{
    color: #ffcc00;
    background: rgba(255, 204, 0, 0.12);
}}

.zenith-todo-badge-low {{
    color: #00ccff;
    background: rgba(0, 204, 255, 0.12);
}}

/* ── Action Buttons (move / delete) ────────────────────────────── */
.zenith-todo-move-btn,
.zenith-todo-del-btn {{
    background: transparent;
    border: none;
    box-shadow: none;
    min-height: 0;
    min-width: 0;
    padding: 2px 6px;
    font-size: 11px;
    border-radius: 4px;
    color: #6e7681;
    transition: all 150ms ease;
}}

.zenith-todo-move-btn:hover {{
    background: rgba(0, 204, 255, 0.12);
    color: #00ccff;
}}

.zenith-todo-del-btn:hover {{
    background: rgba(255, 0, 85, 0.12);
    color: #ff0055;
}}

/* ── Input Row ─────────────────────────────────────────────────── */
.zenith-todo-input-row {{
    padding: 8px 10px 10px 10px;
}}

.zenith-todo-entry {{
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid #30363d;
    border-radius: 8px;
    color: #cdd6f4;
    padding: 6px 10px;
    font-family: "Inter", sans-serif;
    font-size: 13px;
    caret-color: #00ccff;
}}

.zenith-todo-entry:focus {{
    border-color: #7700ff;
    box-shadow: 0 0 0 1px rgba(119, 0, 255, 0.3);
}}

.zenith-todo-add-btn {{
    background: linear-gradient(135deg, #7700ff, #00ccff);
    border: none;
    border-radius: 8px;
    color: #ffffff;
    font-weight: 800;
    font-size: 16px;
    min-width: 34px;
    min-height: 34px;
    padding: 0;
    box-shadow: 0 0 8px rgba(119, 0, 255, 0.3);
    transition: all 200ms ease;
}}

.zenith-todo-add-btn:hover {{
    box-shadow: 0 0 14px rgba(119, 0, 255, 0.5);
}}

/* ── Keyframes ─────────────────────────────────────────────────── */
@keyframes todo-pulse {{
    0%, 100% {{ text-shadow: 0px 0px 8px rgba(0, 204, 255, 0.3); }}
    50%      {{ text-shadow: 0px 0px 16px rgba(0, 204, 255, 0.7); }}
}}

@keyframes todo-urgent {{
    0%, 100% {{ text-shadow: 0px 0px 6px rgba(255, 85, 85, 0.3); }}
    50%      {{ text-shadow: 0px 0px 14px rgba(255, 85, 85, 0.6); }}
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
