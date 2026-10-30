//! The `[bar]` table.

use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Deserializer, de};

/// The `[bar]` table: placement, geometry and motion of every bar.
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema)]
#[serde(default)]
pub struct BarConfig {
    /// Edge of the output the bar sits on.
    pub position: Position,
    /// Outputs that get a bar: `"all"` or a list of connector names such as `["DP-1"]`.
    pub monitors: Monitors,
    /// Bar height in pixels.
    pub height: u32,
    /// Gap between the bar and the output edges, in pixels.
    pub margin: u32,
    /// Corner radius of the bar, in pixels.
    pub radius: u32,
    /// Animation level: `off`, `subtle` (event-driven transitions only) or `ambient`.
    pub motion: Motion,
    /// Module that gets the neon glow, or `"none"`.
    pub focal: String,
}

impl Default for BarConfig {
    fn default() -> Self {
        Self {
            position: Position::Top,
            monitors: Monitors::All,
            height: 34,
            margin: 8,
            radius: 14,
            motion: Motion::Subtle,
            focal: "clock".into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Position {
    Top,
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Motion {
    Off,
    Subtle,
    Ambient,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Monitors {
    All,
    Only(Vec<String>),
}

impl<'de> Deserialize<'de> for Monitors {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Monitors;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("\"all\" or a list of output names")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Monitors, E> {
                match v {
                    "all" => Ok(Monitors::All),
                    _ => Err(E::invalid_value(de::Unexpected::Str(v), &self)),
                }
            }

            fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Monitors, A::Error> {
                let mut names = Vec::new();
                while let Some(name) = seq.next_element()? {
                    names.push(name);
                }
                Ok(Monitors::Only(names))
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

impl JsonSchema for Monitors {
    fn schema_name() -> Cow<'static, str> {
        "Monitors".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "anyOf": [
                { "const": "all" },
                { "type": "array", "items": { "type": "string" } }
            ]
        })
    }
}
