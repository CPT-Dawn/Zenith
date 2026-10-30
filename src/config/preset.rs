//! Built-in presets: a base layer the user's config is merged over.
//!
//! A preset is a complete config file. `init --preset` writes it verbatim, and
//! [`super::resolve`] can lay any config text over it: tables merge key by key,
//! except `[layout]`, which replaces the preset's layout as a whole (merging
//! sections would leave a module in two places).

use std::str::FromStr;

use toml::{Table, Value};

/// A complete starter look: layout, geometry, motion and a theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preset {
    /// Floating Midnight Bloom glass with a glowing clock and subtle motion.
    Bloom,
    /// Flush to the edge, no glow, no motion: the lowest-resource showcase.
    Minimal,
    /// Zenith 1.x's layout in Tokyo Night.
    Classic,
}

impl Preset {
    /// Every built-in preset.
    pub const ALL: [Preset; 3] = [Preset::Bloom, Preset::Minimal, Preset::Classic];

    /// The name `init --preset` takes.
    pub fn as_str(self) -> &'static str {
        match self {
            Preset::Bloom => "bloom",
            Preset::Minimal => "minimal",
            Preset::Classic => "classic",
        }
    }

    /// The preset's config file, as `init --preset` writes it.
    pub fn text(self) -> &'static str {
        match self {
            Preset::Bloom => include_str!("presets/bloom.toml"),
            Preset::Minimal => include_str!("presets/minimal.toml"),
            Preset::Classic => include_str!("presets/classic.toml"),
        }
    }

    /// The preset as a TOML table, ready for [`overlay`].
    pub(super) fn table(self) -> Table {
        toml::from_str(self.text()).expect("built-in presets are valid TOML")
    }
}

impl FromStr for Preset {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Preset::ALL
            .into_iter()
            .find(|preset| preset.as_str() == s)
            .ok_or(())
    }
}

/// Lay `user` over `base`: user values win, tables merge, `[layout]` is replaced whole.
pub(super) fn overlay(base: &mut Table, user: Table) {
    for (key, value) in user {
        match (base.get_mut(&key), value) {
            (Some(Value::Table(base)), Value::Table(user)) if key != "layout" => {
                overlay(base, user);
            }
            (_, value) => {
                base.insert(key, value);
            }
        }
    }
}
