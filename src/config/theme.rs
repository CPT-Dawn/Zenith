//! Theme tokens generated from config, layered above the built-in theme CSS.

use super::resolved::Bar;

/// `:root { --zb-… }` declarations derived from config. `[theme]` overrides append here.
pub fn override_block(bar: &Bar) -> String {
    format!(
        ":root {{\n  --zb-bar-height: {}px;\n  --zb-bar-radius: {}px;\n}}\n",
        bar.height, bar.radius
    )
}
