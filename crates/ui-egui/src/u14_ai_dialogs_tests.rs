//! Step u14 regression tests: AI Settings, Agents & AI Authoring Dialogs (`Area 11`).

use crate::alt_text::AltRow;
use crate::test_harness::{TestHarness, capture_layers, check_contrast, check_nested_scrolls};
use crate::translate_ai::TranslateRow;
use coursecraft_ai::{AiConfig, ElevenLabsModel, ElevenLabsVoice};
use coursecraft_course::Course;
use coursecraft_render::ThemeMode;
use egui::{Key, Modifiers};

fn base_course() -> Course {
    let raw = serde_json::json!({
        "schema_version": 1,
        "id": "c-u14",
        "title": "Step u14 Course",
        "slide_size": { "w": 1280, "h": 720 },
        "masters": [{ "id": "m1", "title": "Default Master", "objects": [] }],
        "scenes": [{
            "id": "scene1",
            "title": "Main",
            "slides": [{
                "id": "s1",
                "title": "Slide 1",
                "duration_ms": 60000,
                "objects": [
                    {
                        "id": "t1",
                        "name": "Text 1",
                        "frame": { "x": 100.0, "y": 100.0, "w": 300.0, "h": 80.0 },
                        "kind": { "type": "text", "text": "Welcome to CourseCraft" }
                    }
                ],
                "layers": [],
                "triggers": []
            }]
        }]
    });
    serde_json::from_value(raw).expect("valid course json")
}

fn configure_ai_ready(h: &mut TestHarness) {
    let cfg = AiConfig {
        base_url: "https://api.openai.com/v1".to_owned(),
        model: "gpt-4o-mini".to_owned(),
        api_key: "sk-test-key".to_owned(),
        speech_to_text_model: "whisper-1".to_owned(),
        text_to_speech_model: "tts-1".to_owned(),
        voice: "alloy".to_owned(),
    };
    h.editor.ui.ai.config = cfg.clone();
    h.editor.ui.ai.saved = cfg;
}

#[test]
fn t11_1_ai_settings_resizable_provider_combo_esc_stacking_and_disabled_test_connection() {
    let mut h = TestHarness::new_with_size(base_course(), (1280.0, 800.0));
    h.editor.open_ai_settings();
    h.editor.ui.ai.tab = crate::ai_settings::AiSettingsTab::Features;
    h.editor.ui.ai.config = AiConfig::default();
    h.idle_frame();
    h.idle_frame();
    h.idle_frame();

    assert!(
        h.find_text("Use:").is_none(),
        "AI Settings 'Use' label must not have a trailing colon"
    );
    assert!(
        h.find_text("Use").is_some(),
        "AI Settings 'Use' label should be present"
    );

    let test_btn = h
        .find_text("Test connection")
        .expect("Test connection button");
    let tip = h.probe_tooltip(test_btn);
    assert!(
        tip.as_deref()
            .is_some_and(|t| t.contains("API URL") && t.contains("model")),
        "Disabled Test connection must explain why: {tip:?}"
    );

    let provider_label = h.editor.ui.ai.provider.label();
    let combo_findings = h.audit_combo(provider_label);
    assert!(
        combo_findings.is_empty(),
        "Provider combo must pass audit_combo: {combo_findings:?}"
    );

    // First Esc closes the open Provider combo only; second Esc closes the dialog.
    let combo_pos = h.find_text(provider_label).expect("Provider combo");
    h.click_at(combo_pos);
    h.idle_frame();
    h.idle_frame();
    h.idle_frame();
    assert!(
        egui::Popup::is_any_open(&h.ctx),
        "Provider combo popup should be open"
    );

    h.press(Key::Escape, Modifiers::NONE);
    assert!(
        !egui::Popup::is_any_open(&h.ctx),
        "First Esc should close Provider combo popup"
    );
    assert!(
        h.editor.ui.ai.open,
        "First Esc must NOT close AI Settings while a combo popup was open"
    );

    h.press(Key::Escape, Modifiers::NONE);
    assert!(!h.editor.ui.ai.open, "Second Esc should close AI Settings");
}

#[test]
fn t11_2_narration_from_text_elevenlabs_combos_and_disabled_generate() {
    let mut h = TestHarness::new_with_size(base_course(), (1280.0, 800.0));
    h.editor.open_speech_dialog();
    h.idle_frame();
    h.idle_frame();
    h.idle_frame();
    let elevenlabs = h.find_text("ElevenLabs").expect("ElevenLabs voice source");
    h.click_at(elevenlabs);
    h.editor.ui.elevenlabs.config.api_key = "el-test-key".to_owned();
    h.editor.ui.elevenlabs.models = vec![
        ElevenLabsModel {
            id: "eleven_multilingual_v2".to_owned(),
            name: "Eleven Multilingual v2".to_owned(),
            max_chars: Some(5000),
        },
        ElevenLabsModel {
            id: "eleven_flash_v2_5".to_owned(),
            name: "Eleven Flash v2.5".to_owned(),
            max_chars: Some(40000),
        },
    ];
    h.editor.ui.elevenlabs.voices = (0..30)
        .map(|i| ElevenLabsVoice {
            id: format!("v-{i}"),
            name: format!("Narrator Voice {i:02}"),
            category: "premade".to_owned(),
        })
        .collect();
    h.editor.ui.elevenlabs.voices_loaded = true;
    h.editor.ui.elevenlabs.config.model_id = "eleven_multilingual_v2".to_owned();
    h.editor.ui.elevenlabs.config.voice_id = "v-0".to_owned();
    h.editor.ui.elevenlabs.config.voice_name = "Narrator Voice 00".to_owned();
    h.editor.ui.speech.text.clear();
    h.idle_frame();
    h.idle_frame();
    h.idle_frame();

    let anchor = h.find_text("Voice source").expect("Voice source");
    h.scroll_over(anchor, -1200.0);
    h.idle_frame();
    let gen_btn = h.find_text("Generate").expect("Generate button");
    h.hover_at(gen_btn);
    h.advance_time(1.0);
    let tip = h.probe_tooltip(gen_btn);
    assert!(
        tip.as_deref().is_some_and(|t| !t.trim().is_empty()),
        "Disabled Generate button must have explanatory tooltip: {tip:?}"
    );
    h.scroll_over(anchor, 1200.0);
    h.idle_frame();

    let model_label = h.editor.ui.elevenlabs.model_label();
    let model_findings = h.audit_combo(&model_label);
    assert!(
        model_findings.is_empty(),
        "ElevenLabs Model combo must pass audit_combo: {model_findings:?}"
    );

    let voice_label = h.editor.ui.elevenlabs.voice_label();
    let voice_findings = h.audit_combo(&voice_label);
    assert!(
        voice_findings.is_empty(),
        "ElevenLabs Voice combo must pass audit_combo: {voice_findings:?}"
    );
}

#[test]
fn t11_3_translate_with_ai_single_scroll_language_combo_colon_free_and_disabled_tooltips() {
    let mut h = TestHarness::new_with_size(base_course(), (1280.0, 800.0));
    configure_ai_ready(&mut h);
    h.editor.open_translate_dialog();
    h.editor.ui.translate.target_language = "fr".to_owned();
    h.editor.ui.translate.rows = (0..20)
        .map(|i| TranslateRow {
            id: format!("u-{i}"),
            location: format!("Slide 1 ▸ Text {i}"),
            source: format!("Source sentence {i}"),
            target: String::new(),
            failure: None,
            problem: None,
        })
        .collect();
    h.idle_frame();
    h.idle_frame();
    h.idle_frame();

    assert!(
        h.find_text("Translate to:").is_none(),
        "'Translate to' must not have a trailing colon"
    );
    assert!(
        h.find_text("Translate to").is_some(),
        "'Translate to' label should be present"
    );

    let layers = capture_layers(&h.ctx);
    let scrolls = crate::scroll_audit::last_recorded_scrolls(&h.ctx);
    let nested = check_nested_scrolls(&layers, &scrolls);
    assert!(
        nested.is_empty(),
        "Translate with AI must have zero nested scroll areas: {nested:?}"
    );

    let combo_findings = h.audit_combo("Common languages");
    assert!(
        combo_findings.is_empty(),
        "Common languages combo must pass audit_combo: {combo_findings:?}"
    );

    h.editor.ui.translate.target_language.clear();
    h.idle_frame();

    let translate_btn = h.find_text("Translate").expect("Translate button");
    let translate_tip = h.probe_tooltip(translate_btn);
    assert!(
        translate_tip
            .as_deref()
            .is_some_and(|t| !t.trim().is_empty()),
        "Disabled Translate button must explain why: {translate_tip:?}"
    );

    let apply_btn = h.find_text("Apply 0").expect("Apply button");
    let apply_tip = h.probe_tooltip(apply_btn);
    assert!(
        apply_tip.as_deref().is_some_and(|t| !t.trim().is_empty()),
        "Disabled Apply button must explain why: {apply_tip:?}"
    );

    // Esc closes open combo first, then dialog.
    let combo_pos = h
        .find_text("Common languages")
        .expect("Common languages combo");
    h.click_at(combo_pos);
    h.idle_frame();
    h.idle_frame();
    h.idle_frame();
    assert!(egui::Popup::is_any_open(&h.ctx));
    h.press(Key::Escape, Modifiers::NONE);
    assert!(!egui::Popup::is_any_open(&h.ctx));
    assert!(
        h.editor.ui.translate.open,
        "First Esc must close combo only, not Translate with AI"
    );
    h.press(Key::Escape, Modifiers::NONE);
    assert!(!h.editor.ui.translate.open);
}

#[test]
fn t11_4_outline_and_document_zero_nested_scroll_fixed_footer_and_esc() {
    let long_outline = (1..=60)
        .map(|i| format!("## Slide {i}\n- Bullet point {i}"))
        .collect::<Vec<_>>()
        .join("\n");

    for size in [(1024.0, 600.0), (1440.0, 900.0)] {
        // Slides from Outline
        let mut h = TestHarness::new_with_size(base_course(), size);
        h.editor.open_outline_dialog();
        h.editor.ui.outline_text = long_outline.clone();
        h.idle_frame();
        h.idle_frame();
        h.idle_frame();

        let layers = capture_layers(&h.ctx);
        let scrolls = crate::scroll_audit::last_recorded_scrolls(&h.ctx);
        let nested = check_nested_scrolls(&layers, &scrolls);
        assert!(
            nested.is_empty(),
            "Slides from Outline at {size:?} must have zero nested scroll areas: {nested:?}"
        );
        assert!(
            h.find_text("Add slides").is_some(),
            "Add slides CTA must be visible at {size:?}"
        );
        h.press(Key::Escape, Modifiers::NONE);
        assert!(!h.editor.ui.show_outline_dialog);

        // Course from Document
        let mut h = TestHarness::new_with_size(base_course(), size);
        configure_ai_ready(&mut h);
        h.editor.open_document_dialog();
        h.editor.ui.document.text = long_outline.clone();
        h.idle_frame();
        h.idle_frame();
        h.idle_frame();

        let layers = capture_layers(&h.ctx);
        let scrolls = crate::scroll_audit::last_recorded_scrolls(&h.ctx);
        let nested = check_nested_scrolls(&layers, &scrolls);
        assert!(
            nested.is_empty(),
            "Course from Document at {size:?} must have zero nested scroll areas: {nested:?}"
        );
        assert!(
            h.find_text("Generate outline").is_some(),
            "Generate outline CTA must be visible at {size:?}"
        );
        h.press(Key::Escape, Modifiers::NONE);
        assert!(!h.editor.ui.document.open);
    }
}

#[test]
fn t11_5_suggest_alt_text_empty_state_disabled_tooltips_and_esc() {
    let mut h = TestHarness::new_with_size(base_course(), (1280.0, 800.0));
    configure_ai_ready(&mut h);
    h.editor.open_alt_text_dialog();
    h.idle_frame();
    h.idle_frame();
    h.idle_frame();

    assert!(
        h.find_text("No images need alt text.").is_some(),
        "Empty state should say no images need alt text"
    );

    h.editor.ui.alt.rows = vec![AltRow {
        slide: "s1".to_owned(),
        object: "img1".to_owned(),
        src: "media/a.png".to_owned(),
        location: "Slide 1 ▸ Image 1".to_owned(),
        suggestion: String::new(),
        ticked: false,
        error: None,
    }];
    h.editor.ui.alt.queue.clear();
    h.idle_frame();
    h.idle_frame();
    h.idle_frame();

    let suggest_btn = h.find_text("Suggest").expect("Suggest button");
    let suggest_tip = h.probe_tooltip(suggest_btn);
    assert!(
        suggest_tip.as_deref().is_some_and(|t| !t.trim().is_empty()),
        "Disabled Suggest button must explain why: {suggest_tip:?}"
    );

    let apply_btn = h.find_text("Apply").expect("Apply button");
    let apply_tip = h.probe_tooltip(apply_btn);
    assert!(
        apply_tip.as_deref().is_some_and(|t| !t.trim().is_empty()),
        "Disabled Apply button must explain why: {apply_tip:?}"
    );

    // Busy state ignores Esc; idle state closes on Esc.
    h.editor.ui.alt.running = true;
    h.idle_frame();
    h.idle_frame();
    h.idle_frame();
    h.press(Key::Escape, Modifiers::NONE);
    assert!(
        h.editor.ui.alt.open,
        "Esc must not close Suggest Alt Text while running"
    );

    h.editor.ui.alt.running = false;
    h.idle_frame();
    h.idle_frame();
    h.idle_frame();
    h.press(Key::Escape, Modifiers::NONE);
    assert!(!h.editor.ui.alt.open);
}

#[test]
fn t11_6_record_narration_device_combo_zero_devices_and_record_contrast() {
    for mode in [ThemeMode::Dark, ThemeMode::Light] {
        let mut h = TestHarness::new_with_theme(base_course(), (1280.0, 800.0), mode);
        h.editor.ui.record_narration.open = true;
        h.editor.ui.record_narration.devices = (1..=5)
            .map(|i| format!("USB Studio Condenser Microphone Channel {i} (48kHz Stereo)"))
            .collect();
        h.idle_frame();
        h.idle_frame();
        h.idle_frame();

        let combo_findings = h.audit_combo("Default");
        assert!(
            combo_findings.is_empty(),
            "Record Narration device combo must pass audit_combo in {mode:?}: {combo_findings:?}"
        );

        let p = coursecraft_render::palette(&h.ctx);
        let layers = capture_layers(&h.ctx);
        let contrast = check_contrast(&layers, &[], &p, &[]);
        assert!(
            contrast.is_empty(),
            "Record Narration contrast violations in {mode:?}: {contrast:?}"
        );

        // 0-device state gives clear actionable message.
        h.editor.ui.record_narration.devices.clear();
        h.idle_frame();
        h.idle_frame();
        h.idle_frame();
        assert!(
            h.find_text_containing("No microphone").is_some(),
            "Zero-device state must show a clear actionable message in {mode:?}"
        );
    }
}

#[test]
fn t11_7_export_translation_and_import_report_single_scroll_and_disabled_export() {
    let mut h = TestHarness::new_with_size(base_course(), (1280.0, 800.0));
    h.editor.ui.show_export_translation = true;
    h.editor.ui.translation_target_language.clear();
    h.idle_frame();
    h.idle_frame();
    h.idle_frame();

    assert!(
        h.find_text("Target language (BCP 47, for example fr or pt-BR):")
            .is_none(),
        "Export Translation label must not have a trailing colon"
    );

    let export_btn = h.find_text("Export…").expect("Export… button");
    let tip = h.probe_tooltip(export_btn);
    assert!(
        tip.as_deref()
            .is_some_and(|t| t.contains("BCP-47") || t.contains("language tag")),
        "Disabled Export… must explain BCP-47 language tag: {tip:?}"
    );

    h.press(Key::Escape, Modifiers::NONE);
    assert!(!h.editor.ui.show_export_translation);

    // Import report with 40 lines
    let report = (1..=40)
        .map(|i| format!("Applied unit {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    h.editor.ui.translation_report = Some(report);
    h.idle_frame();
    h.idle_frame();
    h.idle_frame();

    let layers = capture_layers(&h.ctx);
    let scrolls = crate::scroll_audit::last_recorded_scrolls(&h.ctx);
    let nested = check_nested_scrolls(&layers, &scrolls);
    assert!(
        nested.is_empty(),
        "Translation Import Report must have zero nested scroll areas: {nested:?}"
    );

    h.press(Key::Escape, Modifiers::NONE);
    assert!(
        h.editor.ui.translation_report.is_none(),
        "Translation Import Report must close on Esc"
    );
}

#[test]
fn t11_8_agents_tab_copy_feedback_and_layout_at_1024x600_scale_1_5() {
    for mode in [ThemeMode::Dark, ThemeMode::Light] {
        let mut h = TestHarness::new_with_config(base_course(), (1024.0, 600.0), mode, 1.5, true);
        h.editor.open_ai_settings();
        h.editor.ui.ai.tab = crate::ai_settings::AiSettingsTab::Agents;
        h.editor.automation_available = false;
        h.editor.agent_exe_path = Some(
            "/Users/example/Applications/Very Long Folder Name/CourseCraft.app/Contents/MacOS/CourseCraft"
                .to_owned(),
        );
        h.idle_frame();
        h.idle_frame();
        h.idle_frame();

        let dialog_rect = h
            .ctx
            .memory(|m| m.area_rect(egui::Id::unique(("coursecraft_dialog", "AI settings"))))
            .expect("AI settings dialog rect");
        let mut copy_pos = h.find_text("Copy").expect("Copy button");
        assert!(
            copy_pos.x >= dialog_rect.min.x
                && copy_pos.x <= dialog_rect.max.x
                && h.screen_rect.contains(copy_pos),
            "Copy button must not be pushed horizontally off-screen ({mode:?}): {copy_pos:?} vs {dialog_rect:?}"
        );
        for _ in 0..4 {
            if dialog_rect.contains(copy_pos) {
                break;
            }
            h.scroll_over(dialog_rect.center(), -120.0);
            copy_pos = h.find_text("Copy").expect("Copy button");
        }
        assert!(
            dialog_rect.contains(copy_pos),
            "Copy button should scroll into AI Settings dialog ({mode:?})"
        );
        h.click_at(copy_pos);
        h.idle_frame();
        h.idle_frame();
        h.idle_frame();
        assert!(
            h.find_text("Copied").is_some(),
            "Clicking Copy must show 'Copied' confirmation feedback ({mode:?})"
        );
    }
}

/// L-03: AI, audio, outline, document, and translation dialogs paint sentence-case titles.
#[test]
fn ai_and_translation_dialogs_paint_sentence_case_titles() {
    let mut h = TestHarness::new_with_size(base_course(), (1280.0, 800.0));

    h.editor.open_ai_settings();
    h.idle_frame();
    assert!(
        h.find_text("AI settings").is_some(),
        "expected 'AI settings'"
    );
    assert!(h.find_text("AI Settings").is_none());
    h.editor.ui.ai.open = false;
    h.idle_frame();

    h.editor.open_outline_dialog();
    h.idle_frame();
    assert!(
        h.find_text("Slides from outline").is_some(),
        "expected 'Slides from outline'"
    );
    h.editor.ui.show_outline_dialog = false;
    h.idle_frame();

    h.editor.open_document_dialog();
    h.idle_frame();
    assert!(
        h.find_text("Course from document").is_some(),
        "expected 'Course from document'"
    );
    h.editor.ui.document.open = false;
    h.idle_frame();

    h.editor.open_alt_text_dialog();
    h.idle_frame();
    assert!(
        h.find_text("Suggest alt text").is_some(),
        "expected 'Suggest alt text'"
    );
    h.editor.ui.alt.open = false;
    h.idle_frame();

    h.editor.open_record_narration();
    h.idle_frame();
    assert!(
        h.find_text("Record narration").is_some(),
        "expected 'Record narration'"
    );
    h.editor.ui.record_narration.open = false;
    h.idle_frame();

    h.editor.open_speech_dialog();
    h.idle_frame();
    assert!(
        h.find_text("Narration from text").is_some(),
        "expected 'Narration from text'"
    );
    h.editor.ui.speech.open = false;
    h.idle_frame();

    h.editor.ui.captions.open = true;
    h.idle_frame();
    assert!(
        h.find_text("Generate captions").is_some(),
        "expected 'Generate captions'"
    );
    h.editor.ui.captions.open = false;
    h.idle_frame();

    h.editor.ui.show_export_translation = true;
    h.idle_frame();
    assert!(
        h.find_text("Export translation").is_some(),
        "expected 'Export translation'"
    );
    h.editor.ui.show_export_translation = false;
    h.idle_frame();

    h.editor.set_translation_report("Applied 2 unit(s).");
    h.idle_frame();
    assert!(
        h.find_text("Translation import report").is_some(),
        "expected 'Translation import report'"
    );
}
