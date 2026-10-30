//! Reading and writing the todo file without blocking the main loop (ADR-0002).
//!
//! Saves write a temp file beside the target and rename it over the old one, so a
//! crash leaves either the old file or the new one, never half of either. gio's
//! `replace_contents` alone is not enough: when it cannot create its own temp file
//! it quietly falls back to rewriting the target in place.

use std::path::Path;

use gtk4::gio::prelude::*;
use gtk4::{gio, glib};

/// The file's contents, or `None` if it does not exist.
pub async fn read(path: &Path) -> Result<Option<String>, glib::Error> {
    match gio::File::for_path(path).load_contents_future().await {
        Ok((bytes, _etag)) => Ok(Some(String::from_utf8_lossy(&bytes).into_owned())),
        Err(e) if e.matches(gio::IOErrorEnum::NotFound) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Atomically replace `path` with `contents`, creating missing parent directories.
pub async fn save(path: &Path, contents: String) -> Result<(), glib::Error> {
    if let Some(parent) = path.parent() {
        make_dirs(parent).await?;
    }
    let mut name = std::ffi::OsString::from(".");
    name.push(path.file_name().unwrap_or_default());
    name.push(".tmp");
    let temp = gio::File::for_path(path.with_file_name(name));

    temp.replace_contents_future(contents, None, false, gio::FileCreateFlags::NONE)
        .await
        .map_err(|(_, e)| e)?;
    let (moved, _progress) = temp.move_future(
        &gio::File::for_path(path),
        gio::FileCopyFlags::OVERWRITE | gio::FileCopyFlags::NO_FALLBACK_FOR_MOVE,
        glib::Priority::DEFAULT,
    );
    if let Err(e) = moved.await {
        let _ = temp.delete_future(glib::Priority::DEFAULT).await;
        return Err(e);
    }
    Ok(())
}

/// `mkdir -p` with async gio calls: climb until a directory can be made, then come back down.
async fn make_dirs(dir: &Path) -> Result<(), glib::Error> {
    let mut missing = Vec::new();
    let mut current = Some(dir);
    while let Some(dir) = current {
        match gio::File::for_path(dir)
            .make_directory_future(glib::Priority::DEFAULT)
            .await
        {
            Ok(()) => break,
            Err(e) if e.matches(gio::IOErrorEnum::Exists) => break,
            Err(e) if e.matches(gio::IOErrorEnum::NotFound) => {
                missing.push(dir);
                current = dir.parent();
            }
            Err(e) => return Err(e),
        }
    }
    // The first entry is the deepest; everything above it now exists.
    for dir in missing.into_iter().rev() {
        match gio::File::for_path(dir)
            .make_directory_future(glib::Priority::DEFAULT)
            .await
        {
            Ok(()) => {}
            Err(e) if e.matches(gio::IOErrorEnum::Exists) => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
