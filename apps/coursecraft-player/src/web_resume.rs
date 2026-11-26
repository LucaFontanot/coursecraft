//! Standalone web resume storage using browser `localStorage`.
//!
//! Used only for courses published with `standard: "web"` (never on LMS
//! launches). All storage operations are fallible and ignore errors such as
//! private browsing or quota exhaustion.

#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
#[cfg(any(not(target_arch = "wasm32"), test))]
use std::collections::BTreeMap;
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;

use coursecraft_course::{Course, Issue};
use coursecraft_logic::{Effect, Player, PlayerState, decode_suspend, encode_suspend};
#[cfg(target_arch = "wasm32")]
use eframe::wasm_bindgen::{JsCast as _, JsValue, closure::Closure};

/// Key-value storage backend for standalone web resume data.
pub trait SuspendStorage {
    /// Read the stored suspend string for `key`, if available.
    fn get(&self, key: &str) -> Option<String>;
    /// Save `value` under `key`, ignoring storage errors.
    fn set(&mut self, key: &str, value: &str);
    /// Remove `key` from storage, ignoring storage errors.
    fn remove(&mut self, key: &str);
}

impl<T: SuspendStorage + ?Sized> SuspendStorage for Box<T> {
    fn get(&self, key: &str) -> Option<String> {
        (**self).get(key)
    }

    fn set(&mut self, key: &str, value: &str) {
        (**self).set(key, value);
    }

    fn remove(&mut self, key: &str) {
        (**self).remove(key);
    }
}

/// Storage key for a course's web resume state.
#[must_use]
pub fn storage_key(course: &Course) -> String {
    format!("coursecraft:resume:{}", course.id)
}

/// In-memory [`SuspendStorage`] implementation for tests.
#[cfg(any(not(target_arch = "wasm32"), test))]
#[derive(Debug, Default, Clone)]
pub struct MemoryStorage {
    entries: BTreeMap<String, String>,
}

#[cfg(any(not(target_arch = "wasm32"), test))]
impl MemoryStorage {
    /// Read an entry directly for assertions.
    #[must_use]
    pub fn raw(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(String::as_str)
    }
}

#[cfg(any(not(target_arch = "wasm32"), test))]
impl SuspendStorage for MemoryStorage {
    fn get(&self, key: &str) -> Option<String> {
        self.entries.get(key).cloned()
    }

    fn set(&mut self, key: &str, value: &str) {
        self.entries.insert(key.to_owned(), value.to_owned());
    }

    fn remove(&mut self, key: &str) {
        self.entries.remove(key);
    }
}

/// Browser `localStorage` backend (wasm only).
#[cfg(target_arch = "wasm32")]
#[derive(Debug, Default, Clone, Copy)]
pub struct LocalStorage;

#[cfg(target_arch = "wasm32")]
impl LocalStorage {
    fn storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok().flatten()
    }
}

#[cfg(target_arch = "wasm32")]
impl SuspendStorage for LocalStorage {
    fn get(&self, key: &str) -> Option<String> {
        Self::storage()?.get_item(key).ok().flatten()
    }

    fn set(&mut self, key: &str, value: &str) {
        if let Some(storage) = Self::storage() {
            let _ = storage.set_item(key, value);
        }
    }

    fn remove(&mut self, key: &str) {
        if let Some(storage) = Self::storage() {
            let _ = storage.remove_item(key);
        }
    }
}

/// Coordinates loading, saving, and clearing web resume data for one course.
#[derive(Debug)]
pub struct WebResume<S: SuspendStorage> {
    storage: S,
    key: String,
    last_state: Option<PlayerState>,
    last_saved: Option<String>,
}

impl<S: SuspendStorage> WebResume<S> {
    /// Create a resume coordinator for `course` backed by `storage`.
    #[must_use]
    pub fn new(storage: S, course: &Course) -> Self {
        Self {
            storage,
            key: storage_key(course),
            last_state: None,
            last_saved: None,
        }
    }

    /// Load a player from saved suspend data if present and valid, else start fresh.
    ///
    /// # Errors
    /// Returns validation issues if `course` itself is invalid.
    pub fn load_player<'c>(&self, course: &'c Course, seed: u32) -> Result<Player<'c>, Vec<Issue>> {
        if let Some(raw) = self.storage.get(&self.key)
            && !raw.is_empty()
            && let Ok(decoded) = decode_suspend(course, &raw)
            && let Ok(player) = Player::resume(course, decoded)
        {
            return Ok(player);
        }
        Player::with_draw_seed(course, seed)
    }

    /// Update cached state and persist to storage when the slide changes.
    pub fn after_input(&mut self, course: &Course, state: &PlayerState, effects: &[Effect]) {
        // Most inputs leave the state alone; copy it only when it changed, and
        // into the old copy so its buffers are reused.
        match &mut self.last_state {
            Some(last) if *last == *state => {}
            Some(last) => last.clone_from(state),
            None => self.last_state = Some(state.clone()),
        }
        if effects
            .iter()
            .any(|effect| matches!(effect, Effect::SlideEntered { .. }))
        {
            self.save(course, state);
        }
    }

    /// Persist the latest known state (used on `pagehide`).
    pub fn save_current(&mut self, course: &Course) {
        if let Some(state) = self.last_state.clone() {
            self.save(course, &state);
        }
    }

    /// Clear any saved resume data for this course.
    pub fn clear(&mut self) {
        self.storage.remove(&self.key);
        self.last_saved = None;
        self.last_state = None;
    }

    fn save(&mut self, course: &Course, state: &PlayerState) {
        let encoded = encode_suspend(course, state);
        if self.last_saved.as_deref() != Some(encoded.as_str()) {
            self.storage.set(&self.key, &encoded);
            self.last_saved = Some(encoded);
        }
    }
}

/// Shared handle to the browser `localStorage` resume coordinator.
#[cfg(target_arch = "wasm32")]
pub type WebResumeHandle = Rc<RefCell<WebResume<LocalStorage>>>;

/// Initialize a standalone web player, resuming from `localStorage` if valid.
///
/// # Errors
/// Returns joined validation issues if the course cannot be played.
#[cfg(target_arch = "wasm32")]
pub fn init_web_player(
    course: &'static Course,
) -> Result<(Player<'static>, WebResumeHandle, f64), String> {
    let resume = WebResume::new(LocalStorage, course);
    let mut player = resume
        .load_player(course, crate::lms::fresh_seed())
        .map_err(crate::app::format_issues)?;
    player.set_host_info(coursecraft_logic::HostInfo {
        learner_name: None,
        date: Some(crate::lms::today_date()),
    });
    let handle = Rc::new(RefCell::new(resume));
    register_page_hide(&handle, course);
    Ok((player, handle, crate::lms::now_ms()))
}

/// Record effects and save to `localStorage` after a slide change.
#[cfg(target_arch = "wasm32")]
pub fn after_input(
    handle: &WebResumeHandle,
    course: &Course,
    state: &PlayerState,
    effects: &[Effect],
) {
    if let Ok(mut guard) = handle.try_borrow_mut() {
        guard.after_input(course, state, effects);
    }
}

/// Clear saved `localStorage` resume data for this course.
#[cfg(target_arch = "wasm32")]
pub fn clear(handle: &WebResumeHandle) {
    if let Ok(mut guard) = handle.try_borrow_mut() {
        guard.clear();
    }
}

#[cfg(target_arch = "wasm32")]
fn register_page_hide(handle: &WebResumeHandle, course: &'static Course) {
    let Some(win) = web_sys::window() else {
        return;
    };
    let handle_clone = Rc::clone(handle);
    let closure = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_event| {
        if let Ok(mut guard) = handle_clone.try_borrow_mut() {
            guard.save_current(course);
        }
    }));
    let js_fn: &js_sys::Function = closure.as_ref().unchecked_ref();
    let _ = win.add_event_listener_with_callback("pagehide", js_fn);
    // Intentional leak: listener must remain active until the browser tears down the page.
    closure.forget();
}

#[cfg(test)]
mod tests {
    use super::*;
    use coursecraft_course::{Course, Value};
    use coursecraft_logic::Input;

    const HELLO: &str = include_str!("../../../examples/hello-course/course.json");

    #[test]
    fn web_resume_round_trip_restores_slide_and_variables() {
        let course = Course::from_json(HELLO).expect("hello course");
        let mut storage = MemoryStorage::default();
        let key = storage_key(&course);

        {
            let mut resume = WebResume::new(storage.clone(), &course);
            let mut player = resume.load_player(&course, 1).expect("load player");
            let fx = player.handle(&Input::Start);
            resume.after_input(&course, player.state(), &fx);

            let fx = player.handle(&Input::Next);
            resume.after_input(&course, player.state(), &fx);
            let second_slide = player.current_slide().expect("on second slide").id.clone();
            assert_ne!(second_slide, course.scenes[0].slides[0].id);

            storage = resume.storage;
            assert!(storage.raw(&key).is_some(), "saved after slide change");
        }

        let resume2 = WebResume::new(storage, &course);
        let mut resumed = resume2.load_player(&course, 99).expect("resume player");
        resumed.handle(&Input::Start);
        assert_eq!(
            resumed.current_slide().map(|s| s.id.as_str()),
            Some(course.scenes[0].slides[1].id.as_str())
        );
    }

    #[test]
    fn web_resume_corrupt_data_starts_fresh() {
        let course = Course::from_json(HELLO).expect("hello course");
        let key = storage_key(&course);

        for bad in ["", "not-cc1", "cc1:!!!corrupted!!!"] {
            let mut storage = MemoryStorage::default();
            storage.set(&key, bad);
            let resume = WebResume::new(storage, &course);
            let mut player = resume.load_player(&course, 1).expect("fresh fallback");
            player.handle(&Input::Start);
            assert_eq!(
                player.current_slide().map(|s| s.id.as_str()),
                Some(course.scenes[0].slides[0].id.as_str())
            );
        }
    }

    #[test]
    fn web_resume_clear_removes_saved_data_and_starts_fresh() {
        let course = Course::from_json(HELLO).expect("hello course");
        let key = storage_key(&course);
        let mut resume = WebResume::new(MemoryStorage::default(), &course);
        let mut player = resume.load_player(&course, 1).expect("load");
        let fx = player.handle(&Input::Start);
        resume.after_input(&course, player.state(), &fx);
        let fx = player.handle(&Input::Next);
        resume.after_input(&course, player.state(), &fx);
        assert!(resume.storage.raw(&key).is_some());

        resume.clear();
        assert!(resume.storage.raw(&key).is_none());

        let mut fresh = resume.load_player(&course, 1).expect("fresh");
        fresh.handle(&Input::Start);
        assert_eq!(
            fresh.current_slide().map(|s| s.id.as_str()),
            Some(course.scenes[0].slides[0].id.as_str())
        );
    }

    #[test]
    fn web_resume_page_hide_saves_latest_state() {
        let course = Course::from_json(HELLO).expect("hello course");
        let var_name = course
            .variables
            .first()
            .map(|v| v.name.clone())
            .expect("hello course has a variable");

        let mut resume = WebResume::new(MemoryStorage::default(), &course);
        let mut player = resume.load_player(&course, 1).expect("load");
        let fx = player.handle(&Input::Start);
        resume.after_input(&course, player.state(), &fx);

        let mut state_with_var = player.state().clone();
        state_with_var
            .variables
            .insert(var_name.clone(), Value::Number(42.0));
        resume.after_input(&course, &state_with_var, &[]);
        resume.save_current(&course);

        let mut resumed = resume.load_player(&course, 2).expect("resume");
        resumed.handle(&Input::Start);
        assert_eq!(
            resumed.state().variables.get(&var_name),
            Some(&Value::Number(42.0))
        );
    }
}
