//! M26 bug-hunt test for A7.10: Native playback backend hostile-bytes stress test.

use crate::{MediaKind, create_default_backend};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[test]
fn a7_10_native_playback_backend_rejects_hostile_bytes_safely() {
    let stretch = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/healthy-habits/media/stretch.mp4"
    ))
    .unwrap_or_default();
    let truncated: Vec<u8> = stretch.into_iter().take(128).collect();

    let payloads: [&[u8]; 4] = [
        &[],
        b"not-a-video",
        b"\x00\x00\x00\x18ftypisom\x00\x00\x02\x00",
        &truncated,
    ];

    for (idx, bytes) in payloads.iter().enumerate() {
        let mut backend = create_default_backend();
        backend.play("v", "bad.mp4", MediaKind::Video, Arc::from(*bytes));
        backend.pause("v");
        backend.seek("v", 0);
        backend.seek("v", 5000);

        let start = Instant::now();
        let mut finished = false;
        while start.elapsed() < Duration::from_secs(3) {
            if backend.video_error("v").is_some() || !backend.take_ended().is_empty() {
                finished = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        backend.stop_all();
        assert!(
            finished || start.elapsed() < Duration::from_secs(3),
            "payload {idx} hung"
        );
    }
}
