//! Bug hunt (2026-11-20, M24): nasty request paths and socket behaviour for the
//! "Open in browser" preview server.

use crate::preview_server::{PreviewServer, resolve_zip_path};
use coursecraft_pptx::zip::ZipWriter;
use coursecraft_pptx::zip::write::SimpleFileOptions;
use std::io::{Cursor, Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

fn small_zip() -> Vec<u8> {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = ZipWriter::new(&mut cursor);
        let opts = SimpleFileOptions::default();
        writer.start_file("index.html", opts).expect("start");
        writer.write_all(b"<p>hi</p>").expect("write");
        writer.start_file("media/a b.png", opts).expect("start");
        writer.write_all(b"png").expect("write");
        writer.finish().expect("finish");
    }
    cursor.into_inner()
}

fn get_raw(port: u16, request: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("timeout");
    stream.write_all(request.as_bytes()).expect("write");
    let mut buf = Vec::new();
    let _ = stream.read_to_end(&mut buf);
    String::from_utf8_lossy(&buf).into_owned()
}

#[test]
fn percent_encoded_dot_dot_is_refused() {
    assert_eq!(resolve_zip_path("/%2e%2e/x"), None);
    assert_eq!(resolve_zip_path("/%2E%2E%2Fx"), None);
    assert_eq!(resolve_zip_path("/..%2fx"), None);
    assert_eq!(resolve_zip_path("/%2e%2e%5cx"), None);
}

#[test]
fn nul_and_backslash_are_refused() {
    assert_eq!(resolve_zip_path("/a%00b"), None);
    assert_eq!(resolve_zip_path("/a\\b"), None);
}

#[test]
fn leading_double_slash_never_escapes_the_zip() {
    // Served from memory, so this can only ever be a lookup; it must not panic
    // or be treated as an absolute path.
    let got = resolve_zip_path("//etc/passwd").expect("some path");
    assert!(!got.starts_with('/'), "got {got}");
}

#[test]
fn dot_segments_are_plain_lookups_not_escapes() {
    assert_eq!(resolve_zip_path("/a/./b").as_deref(), Some("a/./b"));
}

#[test]
fn query_string_cannot_smuggle_traversal() {
    assert_eq!(
        resolve_zip_path("/?x=../../etc").as_deref(),
        Some("index.html")
    );
    assert_eq!(
        resolve_zip_path("/index.html?../x").as_deref(),
        Some("index.html")
    );
}

#[test]
fn very_long_path_does_not_panic() {
    let long = format!("/{}", "a/".repeat(20_000));
    assert!(resolve_zip_path(&long).is_some());
    let long_encoded = format!("/{}", "%41".repeat(20_000));
    assert!(resolve_zip_path(&long_encoded).is_some());
}

#[test]
fn truncated_percent_escape_is_kept_literally() {
    assert_eq!(resolve_zip_path("/a%2").as_deref(), Some("a%2"));
    assert_eq!(resolve_zip_path("/a%zz").as_deref(), Some("a%zz"));
}

#[test]
fn server_refuses_traversal_and_serves_index_and_spaced_names() {
    let server = PreviewServer::start_from_zip_bytes(&small_zip()).expect("start");
    let port = server
        .url()
        .trim_start_matches("http://127.0.0.1:")
        .trim_end_matches('/')
        .parse::<u16>()
        .expect("port");

    let trav = get_raw(port, "GET /%2e%2e/x HTTP/1.1\r\nHost: x\r\n\r\n");
    assert!(trav.starts_with("HTTP/1.1 400"), "{trav}");

    let root = get_raw(port, "GET /?v=1 HTTP/1.1\r\nHost: x\r\n\r\n");
    assert!(root.starts_with("HTTP/1.1 200 OK"), "{root}");

    let spaced = get_raw(port, "GET /media/a%20b.png HTTP/1.1\r\nHost: x\r\n\r\n");
    assert!(spaced.starts_with("HTTP/1.1 200 OK"), "{spaced}");
}

#[test]
fn empty_connection_closes_without_response_and_server_keeps_running() {
    let server = PreviewServer::start_from_zip_bytes(&small_zip()).expect("start");
    let port = port_of(&server);
    {
        let _s = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    }
    let root = get_raw(port, "GET / HTTP/1.1\r\n\r\n");
    assert!(root.starts_with("HTTP/1.1 200 OK"), "{root}");
}

#[test]
fn request_without_newline_is_abandoned_within_the_read_timeout() {
    let server = PreviewServer::start_from_zip_bytes(&small_zip()).expect("start");
    let port = port_of(&server);
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream.write_all(b"GET /").expect("write");
    let started = Instant::now();
    let mut buf = Vec::new();
    let _ = stream.read_to_end(&mut buf);
    assert!(
        started.elapsed() < Duration::from_secs(8),
        "server must give up on a stalled request"
    );
}

#[test]
#[ignore = "BUG (low): one stalled connection blocks every other preview request for about 5 s (single-threaded accept loop)"]
fn stalled_client_does_not_block_other_requests() {
    let server = PreviewServer::start_from_zip_bytes(&small_zip()).expect("start");
    let port = port_of(&server);
    let _stalled = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let started = Instant::now();
    let root = get_raw(port, "GET / HTTP/1.1\r\n\r\n");
    assert!(root.starts_with("HTTP/1.1 200 OK"), "{root}");
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "a valid request waited {:?} behind a stalled one",
        started.elapsed()
    );
}

#[test]
fn empty_and_non_get_requests_are_answered_safely() {
    let server = PreviewServer::start_from_zip_bytes(&small_zip()).expect("start");
    let port = port_of(&server);
    let head = get_raw(port, "HEAD / HTTP/1.1\r\n\r\n");
    assert!(head.starts_with("HTTP/1.1 405"), "{head}");
    let bare = get_raw(port, "GET\r\n\r\n");
    assert!(bare.starts_with("HTTP/1.1 400"), "{bare}");
}

#[test]
fn zip_with_leading_slash_entry_is_served_at_root() {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = ZipWriter::new(&mut cursor);
        let opts = SimpleFileOptions::default();
        writer.start_file("/index.html", opts).expect("start");
        writer.write_all(b"ok").expect("write");
        writer.finish().expect("finish");
    }
    let server = PreviewServer::start_from_zip_bytes(&cursor.into_inner()).expect("start");
    let root = get_raw(port_of(&server), "GET / HTTP/1.1\r\n\r\n");
    assert!(root.starts_with("HTTP/1.1 200 OK"), "{root}");
}

#[test]
#[ignore = "BUG (low): a zip entry named ./index.html is not served at the root"]
fn zip_with_dot_slash_entry_is_not_served_at_root() {
    // Entries written as "./index.html" are a common zip-tool quirk. Serving them
    // would be the friendly behaviour; the server currently answers 404.
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = ZipWriter::new(&mut cursor);
        let opts = SimpleFileOptions::default();
        writer.start_file("./index.html", opts).expect("start");
        writer.write_all(b"ok").expect("write");
        writer.finish().expect("finish");
    }
    let server = PreviewServer::start_from_zip_bytes(&cursor.into_inner()).expect("start");
    let root = get_raw(port_of(&server), "GET / HTTP/1.1\r\n\r\n");
    assert!(
        root.starts_with("HTTP/1.1 200 OK"),
        "a './index.html' entry should still be the root page: {root}"
    );
}

fn port_of(server: &PreviewServer) -> u16 {
    server
        .url()
        .trim_start_matches("http://127.0.0.1:")
        .trim_end_matches('/')
        .parse::<u16>()
        .expect("port")
}
