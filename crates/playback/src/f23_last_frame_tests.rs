//! Tests for holding the last decoded video frame after a video ends naturally.

use super::*;

#[test]
fn rodio_backend_holds_last_frame_after_video_ends_until_stopped() {
    let mut backend = RodioBackend::new();
    let frame = Arc::new(VideoFrame {
        pts_ms: 2950,
        width: 8,
        height: 8,
        rgba: vec![255; 8 * 8 * 4],
    });
    let shared = Arc::new(std::sync::Mutex::new(WorkerState {
        frame: Some(Arc::clone(&frame)),
        ended: true,
        ..WorkerState::default()
    }));
    let (cmd_tx, _cmd_rx) = std::sync::mpsc::channel();
    backend.videos.insert(
        "vid1".to_owned(),
        WorkerHandle {
            cmd_tx,
            shared,
            thread: None,
        },
    );

    let ended = backend.take_ended();
    assert_eq!(ended, vec!["vid1"]);
    assert!(
        backend.take_ended().is_empty(),
        "take_ended must report vid1 only once"
    );
    let held = backend
        .video_frame("vid1")
        .expect("last frame must be held after natural video end");
    assert_eq!(held.pts_ms, 2950);

    backend.stop("vid1");
    assert!(
        backend.video_frame("vid1").is_none(),
        "stop must clear the held last frame"
    );
}
