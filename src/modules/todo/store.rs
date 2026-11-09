//! Reading and writing the todo file without blocking the main loop (ADR-0002).
//!
//! Saves write a temp file beside the target and rename it over the old one, so a
//! crash leaves either the old file or the new one, never half of either. gio's
//! `replace_contents` alone is not enough: when it cannot create its own temp file
//! it quietly falls back to rewriting the target in place.

use std::path::{Path, PathBuf};

use gtk4::gio::prelude::*;
use gtk4::{gio, glib};

/// The file's contents, or `None` if it does not exist. A UTF-8 byte-order mark is
/// skipped; other invalid UTF-8 is an `InvalidData` error, never silently repaired.
pub async fn read(path: &Path) -> Result<Option<String>, glib::Error> {
    match gio::File::for_path(path).load_contents_future().await {
        Ok((bytes, _etag)) => {
            let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
            match std::str::from_utf8(bytes) {
                Ok(text) => Ok(Some(text.to_owned())),
                Err(e) => Err(glib::Error::new(
                    gio::IOErrorEnum::InvalidData,
                    &format!("not valid UTF-8 ({e})"),
                )),
            }
        }
        Err(e) if e.matches(gio::IOErrorEnum::NotFound) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Atomically replace `path` with `contents`, creating missing parent directories.
pub async fn save(link: &Path, contents: String) -> Result<(), glib::Error> {
    let resolved = resolve_symlinks(link).await;
    let path = resolved.as_path();
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

/// Follow a chain of symlinks so saves replace the target, not the link itself.
/// A missing or dangling target is returned as is (the save creates it).
async fn resolve_symlinks(path: &Path) -> PathBuf {
    let mut current = path.to_path_buf();
    for _ in 0..40 {
        let info = gio::File::for_path(&current)
            .query_info_future(
                gio::FILE_ATTRIBUTE_STANDARD_SYMLINK_TARGET,
                gio::FileQueryInfoFlags::NOFOLLOW_SYMLINKS,
                glib::Priority::DEFAULT,
            )
            .await;
        let Some(target) = info.ok().and_then(|i| i.symlink_target()) else {
            break;
        };
        current = match current.parent() {
            Some(dir) => dir.join(target),
            None => target,
        };
    }
    current
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

/// Move a corrupt file to `<name>.corrupt` (or `.corrupt.1`, `.corrupt.2`, ...) so the
/// next save cannot destroy it and an earlier backup is never overwritten.
pub async fn set_aside(path: &Path) -> Result<PathBuf, glib::Error> {
    let mut base = path.as_os_str().to_owned();
    base.push(".corrupt");
    let mut n = 0u32;
    loop {
        let mut name = base.clone();
        if n > 0 {
            name.push(format!(".{n}"));
        }
        let target = PathBuf::from(name);
        // No OVERWRITE: if a backup appears under us the move fails rather than clobbers.
        let (moved, _progress) = gio::File::for_path(path).move_future(
            &gio::File::for_path(&target),
            gio::FileCopyFlags::NONE,
            glib::Priority::DEFAULT,
        );
        match moved.await {
            Ok(()) => return Ok(target),
            Err(e) if e.matches(gio::IOErrorEnum::Exists) && n < 1000 => n += 1,
            Err(e) => return Err(e),
        }
    }
}
