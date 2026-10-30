fn main() {
    glib_build_tools::compile_resources(
        &["resources"],
        "resources/zenith-bar.gresource.xml",
        "zenith-bar.gresource",
    );
}
