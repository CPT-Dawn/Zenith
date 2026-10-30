//! Pure: which outputs should have a bar. No GTK here, so it is tested in `tests/placement.rs`.

use crate::config::Monitors;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Changes<K> {
    pub add: Vec<K>,
    pub remove: Vec<K>,
}

/// Compare the outputs that have a bar (`bars`) with the outputs that exist now
/// (`outputs`, each with its connector name if known) and say which bars to add and remove.
/// Bars that should stay are left alone, so a hotplug never rebuilds the other bars.
pub fn reconcile<K: PartialEq + Clone>(
    bars: &[K],
    outputs: &[(K, Option<&str>)],
    monitors: &Monitors,
) -> Changes<K> {
    let wanted: Vec<&K> = outputs
        .iter()
        .filter(|(_, connector)| wanted(monitors, *connector))
        .map(|(key, _)| key)
        .collect();
    Changes {
        add: wanted
            .iter()
            .filter(|key| !bars.contains(key))
            .map(|key| (*key).clone())
            .collect(),
        remove: bars
            .iter()
            .filter(|key| !wanted.contains(key))
            .cloned()
            .collect(),
    }
}

/// An output whose connector name is not known yet matches only `"all"`.
fn wanted(monitors: &Monitors, connector: Option<&str>) -> bool {
    match monitors {
        Monitors::All => true,
        Monitors::Only(names) => connector.is_some_and(|c| names.iter().any(|n| n == c)),
    }
}
