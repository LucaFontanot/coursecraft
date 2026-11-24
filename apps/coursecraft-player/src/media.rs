//! Media sources for desktop and web player.

#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;
#[cfg(any(target_arch = "wasm32", test))]
use std::collections::HashSet;
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Mutex;

use coursecraft_render::{MediaFetch, MediaSource};

/// Desktop media source that loads files from `<course dir>/media/<src>` lazily on request.
#[cfg(not(target_arch = "wasm32"))]
pub struct DesktopMediaSource {
    media_dir: PathBuf,
    cache: Mutex<HashMap<String, Option<Arc<[u8]>>>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl DesktopMediaSource {
    /// Create a new desktop media source rooted at the parent directory of `course_path`.
    pub fn new(course_path: &Path) -> Self {
        let parent = course_path.parent().unwrap_or_else(|| Path::new("."));
        let media_dir = parent.join("media");
        Self {
            media_dir,
            cache: Mutex::new(HashMap::new()),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl MediaSource for DesktopMediaSource {
    fn get(&self, src: &str) -> MediaFetch {
        if !coursecraft_course::is_safe_media_path(src) {
            return MediaFetch::Missing;
        }

        let mut lock = match self.cache.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        if let Some(entry) = lock.get(src) {
            return match entry {
                Some(bytes) => MediaFetch::Ready(Arc::clone(bytes)),
                None => MediaFetch::Missing,
            };
        }

        let full_path = self.media_dir.join(src);
        match std::fs::read(&full_path) {
            Ok(bytes) => {
                let arc: Arc<[u8]> = bytes.into();
                lock.insert(src.to_string(), Some(Arc::clone(&arc)));
                MediaFetch::Ready(arc)
            }
            Err(_) => {
                lock.insert(src.to_string(), None);
                MediaFetch::Missing
            }
        }
    }
}

/// Maximum raw bytes kept in the web media cache before evicting LRU entries
/// that are not on the current slide (64 MiB).
#[cfg(any(target_arch = "wasm32", test))]
pub const MAX_WEB_CACHE_BYTES: usize = 64 * 1024 * 1024;

/// Collect image `src` paths referenced by `slide` (including its layers and
/// master slide, if any). Excludes video and audio.
#[cfg(any(target_arch = "wasm32", test))]
#[must_use]
pub fn slide_image_srcs(
    course: &coursecraft_course::Course,
    slide: &coursecraft_course::Slide,
) -> Vec<String> {
    let mut out = Vec::new();
    collect_slide_srcs(slide, false, &mut out);
    if let Some(master_id) = &slide.master
        && let Some(master) = course.master(master_id)
    {
        collect_slide_srcs(master, false, &mut out);
    }
    out
}

/// Collect all web-fetched media `src` paths referenced by `slide` (images,
/// lottie, captions, and master slide media) so they are protected from LRU
/// eviction while the slide is active.
#[cfg(any(target_arch = "wasm32", test))]
#[must_use]
pub fn slide_media_srcs(
    course: &coursecraft_course::Course,
    slide: &coursecraft_course::Slide,
) -> Vec<String> {
    let mut out = Vec::new();
    collect_slide_srcs(slide, true, &mut out);
    if let Some(master_id) = &slide.master
        && let Some(master) = course.master(master_id)
    {
        collect_slide_srcs(master, true, &mut out);
    }
    out
}

#[cfg(any(target_arch = "wasm32", test))]
fn collect_slide_srcs(
    slide: &coursecraft_course::Slide,
    include_non_image: bool,
    out: &mut Vec<String>,
) {
    for object in &slide.objects {
        collect_object_srcs(object, include_non_image, out);
    }
    for layer in &slide.layers {
        for object in &layer.objects {
            collect_object_srcs(object, include_non_image, out);
        }
    }
}

#[cfg(any(target_arch = "wasm32", test))]
fn collect_object_srcs(
    object: &coursecraft_course::Object,
    include_non_image: bool,
    out: &mut Vec<String>,
) {
    use coursecraft_course::{FillStyle, ObjectKind, StateExtraKind};
    match &object.kind {
        ObjectKind::Image { src, .. } => {
            if !src.is_empty() {
                out.push(src.clone());
            }
        }
        ObjectKind::Shape { fill_style, .. } | ObjectKind::Button { fill_style, .. } => {
            if let Some(src) = fill_style.as_ref().and_then(FillStyle::picture_src)
                && !src.is_empty()
            {
                out.push(src.to_owned());
            }
        }
        ObjectKind::Path { path } => {
            if let Some(src) = path.fill_style.as_ref().and_then(FillStyle::picture_src)
                && !src.is_empty()
            {
                out.push(src.to_owned());
            }
        }
        ObjectKind::Lottie { src, .. } if include_non_image => {
            if !src.is_empty() {
                out.push(src.clone());
            }
        }
        ObjectKind::Audio { captions, .. } => {
            if include_non_image
                && let Some(captions) = captions
                && !captions.is_empty()
            {
                out.push(captions.clone());
            }
        }
        ObjectKind::Video {
            captions, poster, ..
        } => {
            if let Some(poster) = poster
                && !poster.is_empty()
            {
                out.push(poster.clone());
            }
            if include_non_image
                && let Some(captions) = captions
                && !captions.is_empty()
            {
                out.push(captions.clone());
            }
        }
        _ => {}
    }
    for state in &object.states {
        if let Some(state_src) = &state.src
            && !state_src.is_empty()
        {
            out.push(state_src.clone());
        }
        if let Some(src) = state.fill_style.as_ref().and_then(FillStyle::picture_src)
            && !src.is_empty()
        {
            out.push(src.to_owned());
        }
        for extra in &state.extras {
            match &extra.kind {
                StateExtraKind::Image { src, .. } => {
                    if !src.is_empty() {
                        out.push(src.clone());
                    }
                }
                StateExtraKind::Shape { fill_style, .. } => {
                    if let Some(src) = fill_style.as_ref().and_then(FillStyle::picture_src)
                        && !src.is_empty()
                    {
                        out.push(src.to_owned());
                    }
                }
                StateExtraKind::Path { path } => {
                    if let Some(src) = path.fill_style.as_ref().and_then(FillStyle::picture_src)
                        && !src.is_empty()
                    {
                        out.push(src.to_owned());
                    }
                }
                StateExtraKind::Text { .. } => {}
            }
        }
    }
}

/// Web media source that fetches `media/<src>` relative to the page with at most 6 in-flight requests.
#[cfg(target_arch = "wasm32")]
pub struct WebMediaSource {
    state: Arc<std::sync::Mutex<WebMediaState>>,
    ctx: egui::Context,
}

#[cfg(any(target_arch = "wasm32", test))]
pub(crate) struct WebMediaState {
    entries: std::collections::HashMap<String, WebEntryStatus>,
    #[cfg(target_arch = "wasm32")]
    in_flight: usize,
    queue: std::collections::VecDeque<String>,
    protected: HashSet<String>,
    total_bytes: usize,
    max_bytes: usize,
    tick: u64,
}

#[cfg(any(target_arch = "wasm32", test))]
enum WebEntryStatus {
    Ready { bytes: Arc<[u8]>, last_used: u64 },
    Pending,
    Missing,
}

#[cfg(any(target_arch = "wasm32", test))]
impl WebMediaState {
    pub(crate) fn with_max_bytes(max_bytes: usize) -> Self {
        Self {
            entries: std::collections::HashMap::new(),
            #[cfg(target_arch = "wasm32")]
            in_flight: 0,
            queue: std::collections::VecDeque::new(),
            protected: HashSet::new(),
            total_bytes: 0,
            max_bytes,
            tick: 0,
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn is_idle(&self) -> bool {
        self.in_flight == 0 && self.queue.is_empty()
    }

    pub(crate) fn set_protected(&mut self, srcs: &[String]) {
        self.protected = srcs.iter().cloned().collect();
        self.evict_over_cap();
    }

    pub(crate) fn queue_prefetch(&mut self, srcs: &[String]) -> bool {
        let mut queued = false;
        for src in srcs {
            if !coursecraft_course::is_safe_media_path(src) || self.entries.contains_key(src) {
                continue;
            }
            self.entries.insert(src.clone(), WebEntryStatus::Pending);
            self.queue.push_back(src.clone());
            queued = true;
        }
        queued
    }

    pub(crate) fn get_or_queue(&mut self, src: &str) -> (MediaFetch, bool) {
        if !coursecraft_course::is_safe_media_path(src) {
            return (MediaFetch::Missing, false);
        }

        if let Some(status) = self.entries.get_mut(src) {
            return match status {
                WebEntryStatus::Ready { bytes, last_used } => {
                    self.tick = self.tick.saturating_add(1);
                    *last_used = self.tick;
                    (MediaFetch::Ready(Arc::clone(bytes)), false)
                }
                WebEntryStatus::Pending => (MediaFetch::Pending, false),
                WebEntryStatus::Missing => (MediaFetch::Missing, false),
            };
        }

        self.entries
            .insert(src.to_string(), WebEntryStatus::Pending);
        self.queue.push_back(src.to_string());
        (MediaFetch::Pending, true)
    }

    pub(crate) fn insert_ready(&mut self, src: String, bytes: Arc<[u8]>) {
        self.tick = self.tick.saturating_add(1);
        let new_len = bytes.len();
        if let Some(WebEntryStatus::Ready { bytes: old, .. }) = self.entries.insert(
            src,
            WebEntryStatus::Ready {
                bytes,
                last_used: self.tick,
            },
        ) {
            self.total_bytes = self.total_bytes.saturating_sub(old.len());
        }
        self.total_bytes = self.total_bytes.saturating_add(new_len);
        self.evict_over_cap();
    }

    pub(crate) fn insert_missing(&mut self, src: String) {
        if let Some(WebEntryStatus::Ready { bytes: old, .. }) =
            self.entries.insert(src, WebEntryStatus::Missing)
        {
            self.total_bytes = self.total_bytes.saturating_sub(old.len());
        }
    }

    #[cfg(test)]
    pub(crate) fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    #[cfg(test)]
    pub(crate) fn contains(&self, src: &str) -> bool {
        self.entries.contains_key(src)
    }

    fn evict_over_cap(&mut self) {
        while self.total_bytes > self.max_bytes {
            let victim = self
                .entries
                .iter()
                .filter_map(|(key, status)| match status {
                    WebEntryStatus::Ready { last_used, .. } if !self.protected.contains(key) => {
                        Some((*last_used, key.clone()))
                    }
                    _ => None,
                })
                .min_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)))
                .map(|(_, key)| key);

            let Some(key) = victim else {
                break;
            };
            if let Some(WebEntryStatus::Ready { bytes, .. }) = self.entries.remove(&key) {
                self.total_bytes = self.total_bytes.saturating_sub(bytes.len());
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl WebMediaSource {
    /// Maximum concurrent in-flight HTTP requests for media assets.
    pub const MAX_IN_FLIGHT: usize = 6;

    /// Create a new web media source that repaints `ctx` when assets arrive.
    pub fn new(ctx: egui::Context) -> Self {
        Self::with_max_bytes(ctx, MAX_WEB_CACHE_BYTES)
    }

    /// Create a new web media source with a custom raw-byte cache ceiling.
    pub fn with_max_bytes(ctx: egui::Context, max_bytes: usize) -> Self {
        Self {
            state: Arc::new(std::sync::Mutex::new(WebMediaState::with_max_bytes(
                max_bytes,
            ))),
            ctx,
        }
    }

    /// Return `true` when no media fetches are in flight or queued.
    pub fn is_idle(&self) -> bool {
        let s = match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        s.is_idle()
    }

    /// Mark `srcs` as belonging to the current slide so they are never evicted
    /// by the cache cap.
    pub fn set_current_slide_media(&self, srcs: &[String]) {
        let mut s = match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        s.set_protected(srcs);
    }

    /// Queue `srcs` for background fetching if not already cached or pending.
    pub fn prefetch(&self, srcs: &[String]) {
        let queued = {
            let mut s = match self.state.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            s.queue_prefetch(srcs)
        };
        if queued {
            Self::maybe_start_next(&self.state, &self.ctx);
        }
    }

    fn maybe_start_next(state: &Arc<std::sync::Mutex<WebMediaState>>, ctx: &egui::Context) {
        loop {
            let next_src = {
                let mut s = match state.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                if s.in_flight >= Self::MAX_IN_FLIGHT {
                    return;
                }
                let Some(src) = s.queue.pop_front() else {
                    return;
                };
                s.in_flight += 1;
                src
            };

            let state_cb = Arc::clone(state);
            let ctx_cb = ctx.clone();
            let src_cb = next_src.clone();
            let url = format!("media/{next_src}");

            #[allow(unused_mut)]
            let mut request = ehttp::Request::get(url);
            #[cfg(target_arch = "wasm32")]
            {
                request.credentials = ehttp::Credentials::SameOrigin;
            }
            ehttp::fetch(request, move |response| {
                {
                    let mut s = match state_cb.lock() {
                        Ok(guard) => guard,
                        Err(poisoned) => poisoned.into_inner(),
                    };
                    s.in_flight = s.in_flight.saturating_sub(1);
                    match response {
                        Ok(resp) if resp.status == 200 => {
                            s.insert_ready(src_cb, resp.bytes.into());
                        }
                        _ => {
                            s.insert_missing(src_cb);
                        }
                    }
                }
                ctx_cb.request_repaint();
                Self::maybe_start_next(&state_cb, &ctx_cb);
            });
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl MediaSource for WebMediaSource {
    fn get(&self, src: &str) -> MediaFetch {
        let mut s = match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let (fetch, should_start) = s.get_or_queue(src);
        drop(s);
        if should_start {
            Self::maybe_start_next(&self.state, &self.ctx);
        }
        fetch
    }
}
