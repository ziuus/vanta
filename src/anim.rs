//! Adaptive frame scheduling.
//!
//! Widgets that are visibly animating call [`request`] while rendering with
//! the frame rate they need. The event loop reads the highest request after
//! each draw via [`take`] and sleeps accordingly, so a static screen idles at
//! [`IDLE_FPS`] instead of redrawing at the configured rate forever.

use std::sync::atomic::{AtomicU32, Ordering};

/// Redraw rate when nothing on screen is animating. Fast enough for the
/// clock's seconds and fresh sampler data; slow enough to cost ~nothing.
pub static IDLE_FPS: AtomicU32 = AtomicU32::new(2);

pub fn set_idle_fps(fps: u32) {
    IDLE_FPS.store(fps, Ordering::Relaxed);
}

static REQUESTED: AtomicU32 = AtomicU32::new(0);

/// Ask for at least `fps` frames per second for the next frame.
/// `u32::MAX` means "as fast as the user's configured fps allows".
pub fn request(fps: u32) {
    REQUESTED.fetch_max(fps, Ordering::Relaxed);
}

/// Ask for the full configured frame rate.
pub fn request_full() {
    request(u32::MAX);
}

/// Consume this frame's requests and return the fps to use for the next one,
/// clamped to `[IDLE_FPS, max_fps]`.
pub fn take(max_fps: u32) -> u32 {
    REQUESTED.swap(0, Ordering::Relaxed).clamp(
        IDLE_FPS.load(Ordering::Relaxed),
        max_fps.max(IDLE_FPS.load(Ordering::Relaxed)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_when_nothing_requested_and_full_when_requested() {
        let _ = take(60);
        assert_eq!(take(60), IDLE_FPS.load(Ordering::Relaxed));
        request(10);
        request(4);
        assert_eq!(take(60), 10);
        request_full();
        assert_eq!(take(60), 60);
        assert_eq!(take(60), IDLE_FPS.load(Ordering::Relaxed));
    }
}
