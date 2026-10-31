use pretty_assertions::assert_eq;
use zenith_bar::config::{Diagnostic, resolve};
use zenith_bar::modules::system::Metric::*;

fn errors(text: &str) -> Vec<String> {
    match resolve(text) {
        Ok(_) => panic!("expected diagnostics"),
        Err(diagnostics) => diagnostics.iter().map(Diagnostic::to_string).collect(),
    }
}

#[test]
fn system_defaults_show_everything_every_two_seconds() {
    let system = resolve("").unwrap().system;

    assert_eq!(system.metrics, vec![Cpu, Memory, Temperature]);
    assert_eq!(system.interval, 2);
    assert_eq!(system.temperature.warm, 70);
    assert_eq!(system.temperature.hot, 85);
    assert_eq!(system.temperature.sensor, None);
}

#[test]
fn every_system_key_can_be_set() {
    let text = r#"
[system]
metrics = ["temperature", "cpu"]
interval = 5

[system.temperature]
warm = 60
hot = 75
sensor = "k10temp"
"#;
    let system = resolve(text).unwrap().system;

    assert_eq!(system.metrics, vec![Temperature, Cpu]);
    assert_eq!(system.interval, 5);
    assert_eq!(system.temperature.warm, 60);
    assert_eq!(system.temperature.hot, 75);
    assert_eq!(system.temperature.sensor.as_deref(), Some("k10temp"));
}

#[test]
fn interval_below_one_second_is_rejected() {
    assert_eq!(
        errors("[system]\ninterval = 0\n"),
        vec!["2:12: interval must be at least 1 second"]
    );
}

#[test]
fn warm_must_be_below_hot() {
    assert_eq!(
        errors("[system.temperature]\nwarm = 90\nhot = 80\n"),
        vec!["2:8: temperature.warm (90) must be below temperature.hot (80)"]
    );
}

#[test]
fn a_metric_listed_twice_is_rejected() {
    assert_eq!(
        errors("[system]\nmetrics = [\"cpu\", \"cpu\"]\n"),
        vec!["2:11: metric `cpu` is listed twice"]
    );
}

#[test]
fn unknown_metrics_and_keys_are_reported() {
    let unknown_metric = errors("[system]\nmetrics = [\"gpu\"]\n");
    assert_eq!(unknown_metric.len(), 1);
    assert!(unknown_metric[0].starts_with("2:"), "{unknown_metric:?}");
    assert!(unknown_metric[0].contains("gpu"), "{unknown_metric:?}");

    assert_eq!(
        errors("[system.temperature]\nhott = 90\n"),
        vec!["2:1: unknown key `hott` in [system.temperature] (did you mean `hot`?)"]
    );
}
