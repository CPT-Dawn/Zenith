//! Detecting the first presented frame from a client's `WAYLAND_DEBUG=client` log.
//!
//! A frame reaches the compositor when a surface that has a buffer attached is
//! committed, so we watch requests for `wl_surface.attach(<buffer>)` followed by
//! `wl_surface.commit()` on the same surface. The smoke test also watches later
//! frames, layer-shell anchors, and layer surfaces (one per bar) coming and going.

use std::collections::HashSet;

/// A client request the harness cares about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request {
    /// A surface with a buffer attached was committed: a frame reached the compositor.
    Frame,
    /// `zwlr_layer_surface_v1.set_anchor(edges)`; edges is the protocol bitmask.
    Anchor(u32),
    /// `zwlr_layer_shell_v1.get_layer_surface`: a bar was created.
    BarAdded,
    /// `zwlr_layer_surface_v1.destroy`: a bar was removed.
    BarRemoved,
}

impl Request {
    /// Anchored to the bottom edge and not the top (layer-shell: top = 1, bottom = 2).
    pub fn is_bottom_anchor(self) -> bool {
        matches!(self, Request::Anchor(edges) if edges & 2 != 0 && edges & 1 == 0)
    }
}

/// Reads a `WAYLAND_DEBUG=client` log line by line and reports [`Request`]s.
#[derive(Debug, Default)]
pub struct Requests {
    /// Surfaces with a non-null buffer attached but not yet committed.
    pending: HashSet<String>,
    /// Layer surfaces created and not yet destroyed.
    bars: usize,
}

impl Requests {
    /// Bars alive after the lines fed so far.
    pub fn bars(&self) -> usize {
        self.bars
    }

    pub fn feed(&mut self, line: &str) -> Option<Request> {
        let (interface, id, method, args) = request(line)?;
        match (interface, method) {
            ("zwlr_layer_shell_v1", "get_layer_surface") => {
                self.bars += 1;
                Some(Request::BarAdded)
            }
            ("zwlr_layer_surface_v1", "destroy") => {
                self.bars = self.bars.saturating_sub(1);
                Some(Request::BarRemoved)
            }
            ("zwlr_layer_surface_v1", "set_anchor") => args.trim().trim_end_matches(')').trim().parse().ok().map(Request::Anchor),
            ("wl_surface", "attach") if !args.trim_start().starts_with("nil") => {
                self.pending.insert(id.to_owned());
                None
            }
            ("wl_surface", "attach") => {
                self.pending.remove(id);
                None
            }
            ("wl_surface", "commit") if self.pending.remove(id) => Some(Request::Frame),
            _ => None,
        }
    }
}

/// Feed log lines in order; [`FirstFrame::feed`] returns `true` exactly once,
/// on the line that commits the first buffer.
#[derive(Debug, Default)]
pub struct FirstFrame {
    requests: Requests,
    seen: bool,
}

impl FirstFrame {
    pub fn feed(&mut self, line: &str) -> bool {
        if self.seen {
            return false;
        }
        self.seen = self.requests.feed(line) == Some(Request::Frame);
        self.seen
    }
}

/// Split a client request line into (interface, id, method, args).
fn request(line: &str) -> Option<(&str, &str, &str, &str)> {
    // Requests are marked `->`; events (compositor to client) are not.
    let call = line.split_once("-> ")?.1.trim_start();
    let (interface, rest) = call.split_once(['@', '#'])?;
    let (id, rest) = rest.split_once('.')?;
    let (method, args) = rest.split_once('(')?;
    Some((interface, id, method, args))
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

    #[test]
    fn every_buffer_commit_is_a_frame_and_anchors_are_reported() {
        let log = [
            "[ 1.0] {Default Queue}  -> zwlr_layer_surface_v1#21.set_anchor(13)",
            "[ 2.0] {Default Queue}  -> wl_surface#20.attach(wl_buffer#35, 0, 0)",
            "[ 2.1] {Default Queue}  -> wl_surface#20.commit()",
            "[ 3.0] {Default Queue}  -> wl_surface#20.commit()",
            "[ 4.0] {Default Queue}  -> wl_surface#20.attach(wl_buffer#36, 0, 0)",
            "[ 4.1] {Default Queue}  -> wl_surface#20.commit()",
            // As read from the pipe: with its newline.
            "[ 5.0]  -> zwlr_layer_surface_v1@40.set_anchor(14)\n",
            "[ 6.0] zwlr_layer_surface_v1#21.configure(1234, 1920, 34)",
        ];
        let mut requests = Requests::default();
        let seen: Vec<Request> = log.iter().filter_map(|l| requests.feed(l)).collect();
        assert_eq!(seen, vec![Request::Anchor(13), Request::Frame, Request::Frame, Request::Anchor(14)]);
        assert!(Request::Anchor(14).is_bottom_anchor());
        assert!(!Request::Anchor(13).is_bottom_anchor());
    }

    #[test]
    fn layer_surfaces_are_counted_as_bars_come_and_go() {
        let log = [
            "[ 1.0] {Default Queue}  -> zwlr_layer_shell_v1#12.get_layer_surface(new id zwlr_layer_surface_v1#30, wl_surface#29, wl_output#9, 2, \"zenith-bar\")",
            "[ 1.1]  -> zwlr_layer_shell_v1@12.get_layer_surface(new id zwlr_layer_surface_v1@41, wl_surface@40, wl_output@15, 2, \"zenith-bar\")",
            // An event naming a layer surface is not a request.
            "[ 1.2] {Default Queue} zwlr_layer_surface_v1#30.configure(7, 1920, 34)",
            "[ 2.0] {Default Queue}  -> zwlr_layer_surface_v1#30.destroy()",
        ];
        let mut requests = Requests::default();
        let seen: Vec<Request> = log.iter().filter_map(|l| requests.feed(l)).collect();
        assert_eq!(seen, vec![Request::BarAdded, Request::BarAdded, Request::BarRemoved]);
        assert_eq!(requests.bars(), 1);
    }
}
