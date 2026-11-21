//! Thin `wasm-bindgen` wrapper over [`coursecraft_strict_lms`] for `xtask/mock-lms`.
//!
//! Contains zero SCORM validation rules: every call delegates directly to
//! [`StrictScorm12`] or [`StrictScorm2004`].

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]

use coursecraft_strict_lms::{StrictScorm12, StrictScorm2004};
use wasm_bindgen::prelude::wasm_bindgen;

/// Browser-facing wrapper around either `StrictScorm12` or `StrictScorm2004`.
#[wasm_bindgen]
pub struct StrictLmsWasm {
    inner: Engine,
}

enum Engine {
    Scorm12(Box<StrictScorm12>),
    Scorm2004(Box<StrictScorm2004>),
}

#[wasm_bindgen]
impl StrictLmsWasm {
    /// Create a new strict LMS engine (`is_2004 = false` for SCORM 1.2, `true` for SCORM 2004 4th Ed.).
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new(is_2004: bool) -> Self {
        let inner = if is_2004 {
            Engine::Scorm2004(Box::new(StrictScorm2004::new()))
        } else {
            Engine::Scorm12(Box::new(StrictScorm12::new()))
        };
        Self { inner }
    }

    /// Dispatch a SCORM RTE API call (`LMSInitialize`, `LMSSetValue`, `Initialize`, `SetValue`, etc.).
    pub fn dispatch(&mut self, method: &str, arg0: &str, arg1: &str) -> String {
        let args_1 = [arg0];
        let args_2 = [arg0, arg1];
        let args: &[&str] = match method {
            "LMSSetValue" | "SetValue" => &args_2,
            "LMSGetLastError" | "GetLastError" => &[],
            _ => &args_1,
        };
        match &mut self.inner {
            Engine::Scorm12(e) => e.dispatch(method, args),
            Engine::Scorm2004(e) => e.dispatch(method, args),
        }
    }

    /// Simulate browser close + relaunch using persisted attempt data.
    pub fn relaunch(&mut self) {
        match &mut self.inner {
            Engine::Scorm12(e) => e.relaunch(),
            Engine::Scorm2004(e) => e.relaunch(),
        }
    }

    /// Reset all learner data, calls, and violations to a fresh instance.
    pub fn reset(&mut self) {
        self.inner = match &self.inner {
            Engine::Scorm12(_) => Engine::Scorm12(Box::new(StrictScorm12::new())),
            Engine::Scorm2004(_) => Engine::Scorm2004(Box::new(StrictScorm2004::new())),
        };
    }

    /// Flag a violation if the SCO is still `Running` without calling `LMSFinish` / `Terminate`.
    pub fn assert_clean_finish(&mut self) -> bool {
        match &mut self.inner {
            Engine::Scorm12(e) => e.assert_clean_finish(),
            Engine::Scorm2004(e) => e.assert_clean_terminate(),
        }
    }

    /// Current LMS state snapshot as a JSON string.
    #[must_use]
    pub fn snapshot_json(&self) -> String {
        let val = match &self.inner {
            Engine::Scorm12(e) => e.snapshot(),
            Engine::Scorm2004(e) => e.snapshot(),
        };
        serde_json::to_string(&val).unwrap_or_else(|_| "{}".to_owned())
    }

    /// Recorded API calls as a JSON array string.
    #[must_use]
    pub fn calls_json(&self) -> String {
        match &self.inner {
            Engine::Scorm12(e) => serde_json::to_string(e.calls()),
            Engine::Scorm2004(e) => serde_json::to_string(e.calls()),
        }
        .unwrap_or_else(|_| "[]".to_owned())
    }

    /// Recorded MUST/SHALL violations as a JSON array string.
    #[must_use]
    pub fn violations_json(&self) -> String {
        match &self.inner {
            Engine::Scorm12(e) => serde_json::to_string(e.violations()),
            Engine::Scorm2004(e) => serde_json::to_string(e.violations()),
        }
        .unwrap_or_else(|_| "[]".to_owned())
    }

    /// Recorded advisory warnings as a JSON array string.
    #[must_use]
    pub fn warnings_json(&self) -> String {
        match &self.inner {
            Engine::Scorm12(e) => serde_json::to_string(e.warnings()),
            Engine::Scorm2004(e) => serde_json::to_string(e.warnings()),
        }
        .unwrap_or_else(|_| "[]".to_owned())
    }

    /// Export persisted learner attempt state as JSON (for `localStorage`).
    #[must_use]
    pub fn export_persisted_json(&self) -> String {
        match &self.inner {
            Engine::Scorm12(e) => e.export_persisted_json(),
            Engine::Scorm2004(e) => e.export_persisted_json(),
        }
    }

    /// Restore persisted learner attempt state from JSON (`false` if corrupt/mismatched).
    pub fn import_persisted_json(&mut self, json: &str) -> bool {
        match &mut self.inner {
            Engine::Scorm12(e) => e.import_persisted_json(json).is_ok(),
            Engine::Scorm2004(e) => e.import_persisted_json(json).is_ok(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_wrapper_scorm12_parity_and_persistence() {
        let mut wasm = StrictLmsWasm::new(false);
        let mut native = StrictScorm12::new();

        let seq: &[(&str, &str, &str)] = &[
            ("LMSInitialize", "", ""),
            ("LMSSetValue", "cmi.core.lesson_status", "incomplete"),
            ("LMSSetValue", "cmi.core.lesson_location", "s1"),
            ("LMSSetValue", "cmi.suspend_data", "cc1:abc"),
            ("LMSSetValue", "cmi.core.exit", "suspend"),
            ("LMSSetValue", "cmi.core.session_time", "9999:99:99.99"),
            ("LMSGetLastError", "", ""),
            ("LMSGetValue", "cmi.interactions.0.id", ""),
            ("LMSFinish", "", ""),
        ];

        for &(method, a0, a1) in seq {
            let w_res = wasm.dispatch(method, a0, a1);
            let n_res = match method {
                "LMSSetValue" => native.dispatch(method, &[a0, a1]),
                "LMSGetLastError" => native.dispatch(method, &[]),
                _ => native.dispatch(method, &[a0]),
            };
            assert_eq!(w_res, n_res, "method {method}({a0}, {a1})");
        }

        assert_eq!(
            wasm.violations_json(),
            serde_json::to_string(native.violations()).unwrap_or_default()
        );
        assert_eq!(
            wasm.calls_json(),
            serde_json::to_string(native.calls()).unwrap_or_default()
        );

        let saved = wasm.export_persisted_json();
        let mut reloaded = StrictLmsWasm::new(false);
        assert!(reloaded.import_persisted_json(&saved));
        reloaded.relaunch();
        assert_eq!(reloaded.dispatch("LMSInitialize", "", ""), "true");
        assert_eq!(
            reloaded.dispatch("LMSGetValue", "cmi.core.entry", ""),
            "resume"
        );
        assert_eq!(
            reloaded.dispatch("LMSGetValue", "cmi.suspend_data", ""),
            "cc1:abc"
        );
    }

    #[test]
    fn wasm_wrapper_scorm2004_parity_and_persistence() {
        let mut wasm = StrictLmsWasm::new(true);
        let mut native = StrictScorm2004::new();

        let seq: &[(&str, &str, &str)] = &[
            ("Initialize", "", ""),
            ("SetValue", "cmi.completion_status", "incomplete"),
            ("SetValue", "cmi.exit", "timeout"),
            ("GetLastError", "", ""),
            ("SetValue", "cmi.exit", "suspend"),
            ("SetValue", "cmi.score.raw", "250"),
            ("SetValue", "cmi.suspend_data", "cc1:xyz"),
            ("Terminate", "", ""),
        ];

        for &(method, a0, a1) in seq {
            let w_res = wasm.dispatch(method, a0, a1);
            let n_res = match method {
                "SetValue" => native.dispatch(method, &[a0, a1]),
                "GetLastError" => native.dispatch(method, &[]),
                _ => native.dispatch(method, &[a0]),
            };
            assert_eq!(w_res, n_res, "method {method}({a0}, {a1})");
        }

        assert_eq!(
            wasm.violations_json(),
            serde_json::to_string(native.violations()).unwrap_or_default()
        );

        let saved = wasm.export_persisted_json();
        let mut reloaded = StrictLmsWasm::new(true);
        assert!(reloaded.import_persisted_json(&saved));
        reloaded.relaunch();
        assert_eq!(reloaded.dispatch("Initialize", "", ""), "true");
        assert_eq!(reloaded.dispatch("GetValue", "cmi.entry", ""), "resume");
        assert_eq!(
            reloaded.dispatch("GetValue", "cmi.suspend_data", ""),
            "cc1:xyz"
        );
    }
}
