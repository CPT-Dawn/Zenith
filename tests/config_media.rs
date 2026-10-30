use pretty_assertions::assert_eq;
use zenith_bar::config::{Diagnostic, resolve};
use zenith_bar::modules::media::MediaConfig;

#[test]
fn media_defaults_to_title_and_artist_cut_at_forty_characters() {
    let config = resolve("", None).expect("defaults are valid");

    assert_eq!(
        config.media,
        MediaConfig {
            format: "{title} - {artist}".into(),
            max_length: 40,
            hide_when_idle: false,
        }
    );
}

#[test]
fn every_media_key_can_be_set() {
    let text = "[media]\nformat = \"{artist}: {title}\"\nmax_length = 25\nhide_when_idle = true\n";

    let config = resolve(text, None).expect("valid config");

    assert_eq!(
        config.media,
        MediaConfig {
            format: "{artist}: {title}".into(),
            max_length: 25,
            hide_when_idle: true,
        }
    );
}

#[test]
fn an_unknown_placeholder_is_reported_at_the_format() {
    let text = "[media]\nformat = \"{title} - {artsit}\"\n";

    assert_eq!(
        resolve(text, None),
        Err(vec![Diagnostic {
            line: 2,
            column: 10,
            message: "unknown placeholder `{artsit}` in media format, \
                      expected `{title}`, `{artist}` or `{album}`"
                .into(),
            suggestion: None,
        }])
    );
}

#[test]
fn an_unclosed_placeholder_is_reported() {
    let errors = resolve("[media]\nformat = \"{title\"\n", None).unwrap_err();

    assert_eq!(
        errors.iter().map(Diagnostic::to_string).collect::<Vec<_>>(),
        vec!["2:10: unclosed `{` in media format, write `{{` for a literal brace"]
    );
}

#[test]
fn literal_braces_are_written_doubled() {
    let config = resolve("[media]\nformat = \"{{{title}}}\"\n", None).expect("valid config");

    assert_eq!(config.media.format, "{{{title}}}");
}

#[test]
fn a_typo_in_a_media_key_gets_a_suggestion() {
    let errors = resolve("[media]\nhide_when_idel = true\n", None).unwrap_err();

    assert_eq!(errors[0].suggestion.as_deref(), Some("hide_when_idle"));
}
