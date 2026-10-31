use std::ffi::OsStr;

use zenith_bar::ui::renderer;

#[test]
fn bar_renders_with_cairo_unless_the_user_picked_a_renderer() {
    assert_eq!(renderer::default_for(None), Some("cairo"));
    // An empty variable is how shells "unset" in env files; treat it as unset.
    assert_eq!(renderer::default_for(Some(OsStr::new(""))), Some("cairo"));
    assert_eq!(renderer::default_for(Some(OsStr::new("ngl"))), None);
    assert_eq!(renderer::default_for(Some(OsStr::new("vulkan"))), None);
}
