//! Tiny local HTTP server for previewing a just-published zip in the browser.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Cursor, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use coursecraft_pptx::zip::ZipArchive;

const MAX_CONCURRENT_CONNECTIONS: usize = 16;

struct ConnGuard(Arc<AtomicUsize>);

impl Drop for ConnGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

/// A background HTTP server serving the files of one published zip from memory.
///
/// Dropping it stops the background thread and closes the listening socket.
pub struct PreviewServer {
    port: u16,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl PreviewServer {
    /// Read `zip_path` into memory and start serving its files on `127.0.0.1:0`.
    ///
    /// # Errors
    /// Returns a plain message if the zip cannot be read or the socket cannot bind.
    pub fn start_from_file(zip_path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(zip_path)
            .map_err(|e| format!("Couldn't read {}: {e}", zip_path.display()))?;
        Self::start_from_zip_bytes(&bytes)
    }

    /// Parse `zip_bytes` into memory and start serving its files on `127.0.0.1:0`.
    ///
    /// # Errors
    /// Returns a plain message if the zip is invalid or the socket cannot bind.
    pub fn start_from_zip_bytes(zip_bytes: &[u8]) -> Result<Self, String> {
        let files = read_zip_entries(zip_bytes)?;
        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|e| format!("Couldn't start the preview server: {e}"))?;
        let port = listener
            .local_addr()
            .map_err(|e| format!("Couldn't read the preview server port: {e}"))?
            .port();

        let stop = Arc::new(AtomicBool::new(false));
        let stop_bg = Arc::clone(&stop);
        let files = Arc::new(files);
        let active = Arc::new(AtomicUsize::new(0));

        let thread = std::thread::spawn(move || {
            while !stop_bg.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        if stop_bg.load(Ordering::SeqCst) {
                            break;
                        }
                        if active
                            .try_update(Ordering::SeqCst, Ordering::SeqCst, |n| {
                                (n < MAX_CONCURRENT_CONNECTIONS).then_some(n + 1)
                            })
                            .is_ok()
                        {
                            let files = Arc::clone(&files);
                            let guard = ConnGuard(Arc::clone(&active));
                            let _ = std::thread::Builder::new()
                                .name("preview-conn".into())
                                .spawn(move || {
                                    let _guard = guard;
                                    handle_connection(stream, &files);
                                });
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            port,
            stop,
            thread: Some(thread),
        })
    }

    /// The `http://127.0.0.1:<port>/` URL to open in a browser.
    #[must_use]
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}/", self.port)
    }
}

impl Drop for PreviewServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wake the blocking accept() call so the background thread exits immediately.
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

/// Open `url` in the system's default browser without invoking a shell.
///
/// # Errors
/// Returns a plain message if launching the system opener fails.
pub fn open_url_in_browser(url: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = std::process::Command::new("open");
        command.arg(url);
        command
    };

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = std::process::Command::new("cmd");
        command.args(["/C", "start", "", url]);
        command
    };

    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let mut command = {
        let mut command = std::process::Command::new("xdg-open");
        command.arg(url);
        command
    };

    match command.status() {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(format!("Couldn't open the browser: {status}")),
        Err(e) => Err(format!("Couldn't open the browser: {e}")),
    }
}

fn normalize_zip_entry_name(raw: &str) -> Option<String> {
    if raw.ends_with('/') || raw.ends_with('\\') {
        return None;
    }
    let mut s = raw;
    loop {
        if let Some(rest) = s.strip_prefix('/') {
            s = rest;
        } else if let Some(rest) = s.strip_prefix("./") {
            s = rest;
        } else {
            break;
        }
    }
    if s.is_empty() || s.contains("..") || s.contains('\\') || s.contains('\0') {
        return None;
    }
    Some(s.to_string())
}

fn read_zip_entries(zip_bytes: &[u8]) -> Result<HashMap<String, Vec<u8>>, String> {
    let mut archive = ZipArchive::new(Cursor::new(zip_bytes))
        .map_err(|e| format!("Couldn't open the published zip: {e}"))?;
    let mut files = HashMap::new();
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("Couldn't read the published zip: {e}"))?;
        if entry.is_dir() {
            continue;
        }
        let raw_name = entry
            .name()
            .map_err(|e| format!("Couldn't read the published zip: {e}"))?;
        let Some(name) = normalize_zip_entry_name(&raw_name) else {
            continue;
        };
        let mut buf = Vec::new();
        entry
            .read_to_end(&mut buf)
            .map_err(|e| format!("Couldn't read '{name}' from the published zip: {e}"))?;
        files.insert(name, buf);
    }
    Ok(files)
}

/// Map an HTTP request target to a zip entry path, or `None` if the path is unsafe.
///
/// Rejects any path containing `..` or `\`, strips query strings and fragments,
/// and maps `/` to `index.html`.
#[must_use]
pub fn resolve_zip_path(request_target: &str) -> Option<String> {
    let path_only = request_target
        .split(['?', '#'])
        .next()
        .unwrap_or(request_target);
    let decoded = percent_decode(path_only);
    if decoded.contains("..") || decoded.contains('\\') || decoded.contains('\0') {
        return None;
    }
    let trimmed = decoded.trim_start_matches('/');
    if trimmed.is_empty() {
        Some("index.html".to_string())
    } else {
        Some(trimmed.to_string())
    }
}

/// Return the MIME `Content-Type` for a file path inside the zip.
#[must_use]
pub fn content_type_for_path(path: &str) -> &'static str {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    if ext.eq_ignore_ascii_case("html") || ext.eq_ignore_ascii_case("htm") {
        "text/html; charset=utf-8"
    } else if ext.eq_ignore_ascii_case("js") || ext.eq_ignore_ascii_case("mjs") {
        "application/javascript; charset=utf-8"
    } else if ext.eq_ignore_ascii_case("wasm") {
        "application/wasm"
    } else if ext.eq_ignore_ascii_case("json") {
        "application/json; charset=utf-8"
    } else if ext.eq_ignore_ascii_case("css") {
        "text/css; charset=utf-8"
    } else if ext.eq_ignore_ascii_case("png") {
        "image/png"
    } else if ext.eq_ignore_ascii_case("jpg") || ext.eq_ignore_ascii_case("jpeg") {
        "image/jpeg"
    } else if ext.eq_ignore_ascii_case("svg") {
        "image/svg+xml"
    } else if ext.eq_ignore_ascii_case("webp") {
        "image/webp"
    } else if ext.eq_ignore_ascii_case("mp3") {
        "audio/mpeg"
    } else if ext.eq_ignore_ascii_case("mp4") {
        "video/mp4"
    } else if ext.eq_ignore_ascii_case("vtt") {
        "text/vtt; charset=utf-8"
    } else if ext.eq_ignore_ascii_case("xml") {
        "application/xml; charset=utf-8"
    } else {
        "application/octet-stream"
    }
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(hi), Some(lo)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2]))
        {
            out.push((hi << 4) | lo);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn handle_connection(mut stream: TcpStream, files: &HashMap<String, Vec<u8>>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));

    let request_line = {
        let mut reader = BufReader::new(&stream);
        let mut first = String::new();
        if reader.read_line(&mut first).is_err() || first.is_empty() {
            return;
        }
        // Drain headers up to the blank line.
        let mut line = String::new();
        for _ in 0..64 {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) if line == "\r\n" || line == "\n" => break,
                Ok(_) => {}
            }
        }
        first
    };

    let mut parts = request_line.split_whitespace();
    let Some(method) = parts.next() else {
        write_error(&mut stream, "400 Bad Request", b"Bad Request");
        return;
    };
    let Some(target) = parts.next() else {
        write_error(&mut stream, "400 Bad Request", b"Bad Request");
        return;
    };

    if method != "GET" {
        let header = "HTTP/1.1 405 Method Not Allowed\r\nAllow: GET\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: 18\r\nConnection: close\r\n\r\nMethod Not Allowed";
        let _ = stream.write_all(header.as_bytes());
        let _ = stream.flush();
        return;
    }

    let Some(zip_path) = resolve_zip_path(target) else {
        write_error(&mut stream, "400 Bad Request", b"Bad Request");
        return;
    };

    let Some(body) = files.get(&zip_path) else {
        write_error(&mut stream, "404 Not Found", b"Not Found");
        return;
    };

    let content_type = content_type_for_path(&zip_path);
    let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

fn write_error(stream: &mut TcpStream, status: &str, body: &[u8]) {
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

#[cfg(test)]
mod tests {
    use super::*;
    use coursecraft_pptx::zip::ZipWriter;
    use coursecraft_pptx::zip::write::SimpleFileOptions;

    #[test]
    fn resolve_zip_path_maps_root_and_rejects_dot_dot() {
        assert_eq!(resolve_zip_path("/"), Some("index.html".to_string()));
        assert_eq!(resolve_zip_path(""), Some("index.html".to_string()));
        assert_eq!(
            resolve_zip_path("/index.html?v=1#top"),
            Some("index.html".to_string())
        );
        assert_eq!(resolve_zip_path("/a.wasm"), Some("a.wasm".to_string()));
        assert_eq!(
            resolve_zip_path("/media/pic.png"),
            Some("media/pic.png".to_string())
        );

        assert_eq!(resolve_zip_path("/../secret"), None);
        assert_eq!(resolve_zip_path("/a/../b"), None);
        assert_eq!(resolve_zip_path(".."), None);
        assert_eq!(resolve_zip_path("/%2e%2e/secret"), None);
    }

    #[test]
    fn content_type_covers_all_required_extensions() {
        assert_eq!(
            content_type_for_path("index.html"),
            "text/html; charset=utf-8"
        );
        assert_eq!(
            content_type_for_path("app.js"),
            "application/javascript; charset=utf-8"
        );
        assert_eq!(content_type_for_path("a.wasm"), "application/wasm");
        assert_eq!(
            content_type_for_path("course.json"),
            "application/json; charset=utf-8"
        );
        assert_eq!(
            content_type_for_path("style.css"),
            "text/css; charset=utf-8"
        );
        assert_eq!(content_type_for_path("pic.png"), "image/png");
        assert_eq!(content_type_for_path("photo.jpg"), "image/jpeg");
        assert_eq!(content_type_for_path("icon.svg"), "image/svg+xml");
        assert_eq!(content_type_for_path("hero.webp"), "image/webp");
        assert_eq!(content_type_for_path("voice.mp3"), "audio/mpeg");
        assert_eq!(content_type_for_path("clip.mp4"), "video/mp4");
        assert_eq!(
            content_type_for_path("captions.vtt"),
            "text/vtt; charset=utf-8"
        );
        assert_eq!(
            content_type_for_path("imsmanifest.xml"),
            "application/xml; charset=utf-8"
        );
        assert_eq!(
            content_type_for_path("archive.bin"),
            "application/octet-stream"
        );
        assert_eq!(
            content_type_for_path("no_extension"),
            "application/octet-stream"
        );
    }

    #[test]
    fn server_serves_root_and_wasm_from_in_memory_zip() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = ZipWriter::new(&mut cursor);
            let opts = SimpleFileOptions::default();
            writer.start_file("index.html", opts).unwrap();
            writer.write_all(b"<h1>Hello CourseCraft</h1>").unwrap();
            writer.start_file("a.wasm", opts).unwrap();
            writer.write_all(b"\0asm\x01\x00\x00\x00").unwrap();
            writer.finish().unwrap();
        }
        let zip_bytes = cursor.into_inner();
        let server = PreviewServer::start_from_zip_bytes(&zip_bytes).expect("server starts");

        let resp_root = http_get(server.port, "/");
        assert!(resp_root.starts_with("HTTP/1.1 200 OK\r\n"), "{resp_root}");
        assert!(
            resp_root.contains("Content-Type: text/html; charset=utf-8\r\n"),
            "{resp_root}"
        );
        assert!(resp_root.ends_with("<h1>Hello CourseCraft</h1>"));

        let resp_wasm = http_get(server.port, "/a.wasm");
        assert!(resp_wasm.starts_with("HTTP/1.1 200 OK\r\n"), "{resp_wasm}");
        assert!(
            resp_wasm.contains("Content-Type: application/wasm\r\n"),
            "{resp_wasm}"
        );

        let resp_traversal = http_get(server.port, "/../secret");
        assert!(
            resp_traversal.starts_with("HTTP/1.1 400 Bad Request\r\n"),
            "{resp_traversal}"
        );
    }

    fn http_get(port: u16, path: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
        let req = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n");
        stream.write_all(req.as_bytes()).expect("write");
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).expect("read");
        String::from_utf8_lossy(&buf).into_owned()
    }
}
