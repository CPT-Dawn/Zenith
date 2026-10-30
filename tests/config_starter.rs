use pretty_assertions::assert_eq;
use zenith_bar::config::{resolve, starter};

#[test]
fn the_starter_config_spells_out_the_defaults() {
    let written = resolve(starter(None), None).expect("starter config is valid");

    assert_eq!(written, resolve("", None).unwrap());
}

#[test]
fn the_starter_config_points_editors_at_the_schema() {
    assert_eq!(
        starter(None).lines().next(),
        Some(
            "#:schema https://raw.githubusercontent.com/CPT-Dawn/Zenith/main/schema/config.schema.json"
        )
    );
}
