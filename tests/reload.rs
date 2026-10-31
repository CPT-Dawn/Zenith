//! Hot reload: what a saved config means for the running bar.

use std::path::{Path, PathBuf};

use pretty_assertions::assert_eq;
use zenith_bar::config::{ResolvedConfig, resolve};
use zenith_bar::ui::reload::{File, Outcome, Watched, evaluate, plan};
use zenith_bar::ui::sources::follow;

fn ok(text: &str) -> ResolvedConfig {
    match resolve(text) {
        Ok(config) => config,
        Err(diagnostics) => panic!("expected a valid config, got {diagnostics:#?}"),
    }
}

#[test]
fn events_are_matched_to_the_watched_file_by_full_path() {
    let watched = Watched::new(
        Path::new("/cfg/zenith-bar/config.toml"),
        Path::new("/cfg/zenith-bar/style.css"),
    );

    assert_eq!(
        watched.kind_of(Path::new("/cfg/zenith-bar/config.toml")),
        Some(File::Config)
    );
    assert_eq!(
        watched.kind_of(Path::new("/cfg/zenith-bar/style.css")),
        Some(File::Style)
    );
    // Editor swap and backup files share the directory but are not ours.
    assert_eq!(
        watched.kind_of(Path::new("/cfg/zenith-bar/.config.toml.swp")),
        None
    );
    assert_eq!(watched.kind_of(Path::new("/elsewhere/config.toml")), None);
}

#[test]
fn files_in_one_directory_need_one_directory_watch() {
    let watched = Watched::new(
        Path::new("/cfg/zenith-bar/config.toml"),
        Path::new("/cfg/zenith-bar/style.css"),
    );

    assert_eq!(watched.dirs(), [PathBuf::from("/cfg/zenith-bar")]);
}

#[test]
fn a_symlinked_config_is_watched_where_it_really_lives() {
    let root = scratch("symlink");
    std::fs::create_dir_all(root.join("dotfiles")).unwrap();
    std::fs::create_dir_all(root.join("config")).unwrap();
    std::fs::write(root.join("dotfiles/bar.toml"), "").unwrap();
    std::os::unix::fs::symlink(
        root.join("dotfiles/bar.toml"),
        root.join("config/config.toml"),
    )
    .unwrap();

    let watched = Watched::new(
        &root.join("config/config.toml"),
        &root.join("config/style.css"),
    );

    assert_eq!(
        watched.kind_of(&root.join("dotfiles/bar.toml")),
        Some(File::Config)
    );
    assert_eq!(watched.dirs(), [root.join("config"), root.join("dotfiles")]);
}

#[test]
fn a_missing_config_directory_is_watched_from_its_nearest_existing_parent() {
    let watched = Watched::new(
        Path::new("/home/me/.config/zenith-bar/config.toml"),
        Path::new("/home/me/.config/zenith-bar/style.css"),
    );
    let exists = |dir: &Path| dir == Path::new("/home/me/.config") || dir.ends_with("me");

    assert_eq!(
        watched.watch_points(exists),
        [PathBuf::from("/home/me/.config")]
    );
    assert_eq!(
        watched.watch_points(|_| true),
        [PathBuf::from("/home/me/.config/zenith-bar")]
    );
}

#[test]
fn creating_or_removing_a_directory_on_the_way_means_watching_again() {
    let watched = Watched::new(
        Path::new("/home/me/.config/zenith-bar/config.toml"),
        Path::new("/home/me/.config/zenith-bar/style.css"),
    );

    assert!(watched.is_on_the_way(Path::new("/home/me/.config/zenith-bar")));
    assert!(watched.is_on_the_way(Path::new("/home/me/.config")));
    assert!(!watched.is_on_the_way(Path::new("/home/me/.config/hypr")));
    assert!(!watched.is_on_the_way(Path::new("/home/me/.config/zenith-bar/config.toml")));
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join(format!("zenith-bar-test-{}", std::process::id()))
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir.canonicalize().unwrap()
}

#[test]
fn an_invalid_edit_reports_line_and_column_and_applies_nothing() {
    let running = ok("[theme]\naccent = \"#ff79c6\"\n");

    let outcome = evaluate("[bar]\nheigth = 40\n", &running);

    let Outcome::Invalid(diagnostics) = outcome else {
        panic!("expected diagnostics, got {outcome:?}");
    };
    assert_eq!(
        diagnostics
            .iter()
            .map(|d| d.render("config.toml"))
            .collect::<Vec<_>>(),
        ["config.toml:2:1: unknown key `heigth` in [bar] (did you mean `height`?)"]
    );
}

#[test]
fn a_save_without_a_real_change_is_unchanged() {
    let running = ok("[bar]\nheight = 40\n");

    assert_eq!(
        evaluate("# tweaked a comment\n[bar]\nheight = 40\n", &running),
        Outcome::Unchanged
    );
}

#[test]
fn a_valid_edit_yields_the_new_config() {
    let running = ok("");

    assert_eq!(
        evaluate("[bar]\nheight = 40\n", &running),
        Outcome::Changed(Box::new(ok("[bar]\nheight = 40\n")))
    );
}

#[test]
fn an_accent_change_restyles_without_rebuilding_bars() {
    let old = ok("");
    let new = ok("[theme]\naccent = \"#ff79c6\"\n");

    assert_eq!(plan(&old, &new).rebuild_bars, false);
}

#[test]
fn changes_the_bar_window_is_built_from_rebuild_bars() {
    let old = ok("");
    for text in [
        "[bar]\nposition = \"bottom\"\n",
        "[bar]\nmargin = 0\n",
        "[bar]\nmonitors = [\"DP-1\"]\n",
        "[bar]\nfocal = \"none\"\n",
        "[layout]\nright = [\"system\"]\n",
        "[clock]\non_click = \"foot\"\n",
    ] {
        assert_eq!(plan(&old, &ok(text)).rebuild_bars, true, "{text}");
    }
}

#[test]
fn changes_carried_by_css_or_sources_keep_the_bars() {
    let old = ok("");
    for text in [
        "[bar]\nheight = 40\n",
        "[bar]\nradius = 0\n",
        "[bar]\nmotion = \"off\"\n",
        "[theme]\nsurface_opacity = 0.5\n",
        "[clock]\nformat = \"%H:%M:%S\"\n",
        "[system]\ninterval = 5\n",
        "[media]\nmax_length = 20\n",
    ] {
        assert_eq!(plan(&old, &ok(text)).rebuild_bars, false, "{text}");
    }
}

/// A stand-in source: remembers the config it was started or reconfigured with.
struct Fake(std::cell::Cell<u32>);

fn start(config: u32) -> impl FnOnce() -> std::rc::Rc<Fake> {
    move || std::rc::Rc::new(Fake(config.into()))
}

#[test]
fn a_module_entering_the_layout_starts_its_source_and_leaving_drops_it() {
    let mut slot = None;

    follow(&mut slot, true, false, start(1), |_| unreachable!());
    assert_eq!(slot.as_ref().map(|s| s.0.get()), Some(1));

    follow(&mut slot, false, true, start(2), |_| unreachable!());
    assert!(slot.is_none());
}

#[test]
fn a_running_source_is_reconfigured_in_place_only_when_its_config_changed() {
    let mut slot = Some(start(1)());
    let running = slot.clone().unwrap();

    follow(&mut slot, true, false, start(9), |s| s.0.set(9));
    assert_eq!(running.0.get(), 1);

    follow(&mut slot, true, true, start(9), |s| s.0.set(2));
    assert_eq!(running.0.get(), 2);
    assert!(std::rc::Rc::ptr_eq(&running, slot.as_ref().unwrap()));
}
