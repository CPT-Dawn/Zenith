//! Built-in presets: complete config files that `init --preset` writes verbatim.

use std::str::FromStr;

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
