//! The MPRIS parsers must shrug off whatever any process puts on the session bus.

use gtk4::glib::{self, prelude::ToVariant};
use zenith_bar::modules::media::source::{parse_get, parse_get_all, parse_properties_changed};

fn empty_map() -> glib::Variant {
    glib::Variant::array_from_iter_with_type(
        glib::VariantTy::new("{sv}").unwrap(),
        std::iter::empty::<glib::Variant>(),
    )
}

#[test]
fn a_properties_changed_with_a_short_body_is_ignored() {
    let body = ("org.mpris.MediaPlayer2.Player",).to_variant();
    assert!(parse_properties_changed(&body).is_none());
    assert!(parse_properties_changed(&().to_variant()).is_none());
}

#[test]
fn a_properties_changed_with_the_wrong_types_is_ignored() {
    let body =
        glib::Variant::tuple_from_iter(["x".to_variant(), empty_map(), "notarray".to_variant()]);
    assert!(parse_properties_changed(&body).is_none());
}

#[test]
fn a_well_formed_properties_changed_parses() {
    let none: Vec<String> = vec![];
    let body = glib::Variant::tuple_from_iter(["x".to_variant(), empty_map(), none.to_variant()]);
    let (changed, invalidated) = parse_properties_changed(&body).unwrap();
    assert!(changed.is_empty());
    assert!(!invalidated);
    let some = vec!["Metadata".to_string()];
    let body = glib::Variant::tuple_from_iter(["x".to_variant(), empty_map(), some.to_variant()]);
    assert!(parse_properties_changed(&body).unwrap().1);
}

#[test]
fn a_get_all_reply_of_the_wrong_shape_is_ignored() {
    assert!(parse_get_all(&().to_variant()).is_none());
    assert!(parse_get_all(&("x",).to_variant()).is_none());
    assert!(parse_get_all(&glib::Variant::tuple_from_iter([empty_map()])).is_some());
}

#[test]
fn a_get_reply_of_the_wrong_shape_is_ignored() {
    assert!(parse_get(&().to_variant()).is_none());
    assert!(parse_get(&(5i64,).to_variant()).is_none());
    assert!(parse_get(&(5i64.to_variant().to_variant(),).to_variant()).is_some());
}
