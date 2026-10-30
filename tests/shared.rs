//! One source feeds every bar: views subscribe to a `Shared` and leave when their bar goes.

use std::cell::RefCell;
use std::rc::Rc;

use zenith_bar::modules::Shared;

#[test]
fn every_bar_sees_one_publish_and_a_removed_bar_sees_no_more() {
    let source = Shared::new(0);
    let seen: Rc<RefCell<Vec<(&str, i32)>>> = Rc::default();
    let view = |bar: &'static str| {
        let seen = seen.clone();
        move |value: &i32| seen.borrow_mut().push((bar, *value))
    };

    let _dp1 = source.subscribe(view("DP-1"));
    let hdmi = source.subscribe(view("HDMI-A-1"));
    source.set(1);
    source.unsubscribe(hdmi);
    source.set(2);

    assert_eq!(
        *seen.borrow(),
        [
            ("DP-1", 0),
            ("HDMI-A-1", 0),
            ("DP-1", 1),
            ("HDMI-A-1", 1),
            ("DP-1", 2),
        ]
    );
}
