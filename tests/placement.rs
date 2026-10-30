//! Which outputs get a bar, and what changes when outputs come and go.

use pretty_assertions::assert_eq;
use zenith_bar::config::Monitors;
use zenith_bar::ui::placement::{Changes, reconcile};

fn only(names: &[&str]) -> Monitors {
    Monitors::Only(names.iter().map(|n| n.to_string()).collect())
}

#[test]
fn all_puts_a_bar_on_every_output() {
    let outputs = [(1, Some("DP-1")), (2, Some("HDMI-A-1"))];

    assert_eq!(
        reconcile(&[], &outputs, &Monitors::All),
        Changes {
            add: vec![1, 2],
            remove: vec![]
        }
    );
}

#[test]
fn a_monitor_list_puts_bars_only_on_named_outputs() {
    let outputs = [(1, Some("DP-1")), (2, Some("HDMI-A-1")), (3, Some("eDP-1"))];

    assert_eq!(
        reconcile(&[], &outputs, &only(&["eDP-1", "DP-1"])),
        Changes {
            add: vec![1, 3],
            remove: vec![]
        }
    );
}

#[test]
fn an_output_without_a_connector_name_matches_all_but_no_list() {
    let outputs = [(1, None)];

    assert_eq!(reconcile(&[], &outputs, &Monitors::All).add, vec![1]);
    assert_eq!(
        reconcile(&[], &outputs, &only(&["DP-1"])).add,
        Vec::<i32>::new()
    );
}

#[test]
fn a_newly_connected_output_gets_a_bar_and_existing_bars_stay() {
    let outputs = [(1, Some("DP-1")), (2, Some("HDMI-A-1"))];

    assert_eq!(
        reconcile(&[1], &outputs, &Monitors::All),
        Changes {
            add: vec![2],
            remove: vec![]
        }
    );
}

#[test]
fn a_disconnected_output_loses_its_bar() {
    let outputs = [(2, Some("HDMI-A-1"))];

    assert_eq!(
        reconcile(&[1, 2], &outputs, &Monitors::All),
        Changes {
            add: vec![],
            remove: vec![1]
        }
    );
}

#[test]
fn an_output_that_stops_matching_loses_its_bar() {
    // A connector name can arrive after the output itself.
    let outputs = [(1, Some("HDMI-A-1"))];

    assert_eq!(
        reconcile(&[1], &outputs, &only(&["DP-1"])),
        Changes {
            add: vec![],
            remove: vec![1]
        }
    );
}
