//! Detecting the first presented frame from a client's `WAYLAND_DEBUG=client` log.
//!
//! A frame reaches the compositor when a surface that has a buffer attached is
//! committed, so we watch requests for `wl_surface.attach(<buffer>)` followed by
//! `wl_surface.commit()` on the same surface.

use std::collections::HashSet;

/// Feed log lines in order; [`FirstFrame::feed`] returns `true` exactly once,
/// on the line that commits the first buffer.
#[derive(Debug, Default)]
pub struct FirstFrame {
    /// Surfaces with a non-null buffer attached but not yet committed.
    pending: HashSet<String>,
    seen: bool,
}

impl FirstFrame {
    pub fn feed(&mut self, line: &str) -> bool {
        if self.seen {
            return false;
        }
        let Some((surface, method, args)) = surface_request(line) else {
            return false;
        };
        match method {
            "attach" if !args.trim_start().starts_with("nil") => {
                self.pending.insert(surface.to_owned());
            }
            "attach" => {
                self.pending.remove(surface);
            }
            "commit" if self.pending.contains(surface) => {
                self.seen = true;
                return true;
            }
            _ => {}
        }
        false
    }
}

/// Split a client request line on a `wl_surface` into (id, method, args).
fn surface_request(line: &str) -> Option<(&str, &str, &str)> {
    // Requests are marked `->`; events (compositor to client) are not.
    let call = line.split_once("-> ")?.1.trim_start();
    let rest = call.strip_prefix("wl_surface")?;
    let rest = rest.strip_prefix('@').or_else(|| rest.strip_prefix('#'))?;
    let (id, rest) = rest.split_once('.')?;
    let (method, args) = rest.split_once('(')?;
    Some((id, method, args))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn first_frame_line(lines: &[&str]) -> Option<usize> {
        let mut detector = FirstFrame::default();
        lines.iter().position(|l| detector.feed(l))
    }

    #[test]
    fn commit_after_attaching_a_buffer_is_the_first_frame() {
        // libwayland >= 1.23 format: `#` ids and a queue name.
        let log = [
            "[1084512.301] {Default Queue}  -> wl_compositor#4.create_surface(new id wl_surface#20)",
            "[1084512.402] {Default Queue}  -> zwlr_layer_surface_v1#21.set_size(0, 34)",
            "[1084512.410] {Default Queue}  -> wl_surface#20.commit()",
            "[1084513.020] {Default Queue} zwlr_layer_surface_v1#21.configure(1234, 1920, 34)",
            "[1084530.777] {Default Queue}  -> wl_surface#20.attach(wl_buffer#35, 0, 0)",
            "[1084530.790] {Default Queue}  -> wl_surface#20.damage_buffer(0, 0, 1920, 34)",
            "[1084530.801] {Default Queue}  -> wl_surface#20.commit()",
        ];
        assert_eq!(first_frame_line(&log), Some(6));
    }

    #[test]
    fn null_buffers_and_other_surfaces_do_not_count() {
        // Older libwayland format: `@` ids, no queue name.
        let log = [
            "[ 512.100]  -> wl_surface@20.attach(nil, 0, 0)",
            "[ 512.110]  -> wl_surface@20.commit()",
            "[ 512.200]  -> wl_surface@22.attach(wl_buffer@40, 0, 0)",
            "[ 512.210]  -> wl_surface@20.commit()",
            "[ 512.300] wl_buffer@40.release()",
            "[ 512.400]  -> wl_surface@22.commit()",
        ];
        assert_eq!(first_frame_line(&log), Some(5));
    }
}
