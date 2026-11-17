//! Desktop-only image paste: read the system clipboard on Cmd/Ctrl+V and hand
//! a pasted screenshot to the editor. Text and object paste stay with egui and
//! [`coursecraft_ui_egui::clipboard`].

use arboard::ImageData;
use coursecraft_ui_egui::coursecraft_media::encode_png_rgba;
use coursecraft_ui_egui::{
    Editor,
    clipboard::{CLIPBOARD_KEY, Clipboard},
};

/// What a desktop Cmd/Ctrl+V should do with the current clipboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasteImageAction {
    /// The clipboard holds our object JSON; the normal object paste handles it.
    LeaveToObjectPaste,
    /// Encode the clipboard image and insert it as a picture.
    PasteImage,
    /// No image to paste and no payload to leave alone.
    DoNothing,
}

/// Recognise a CourseCraft payload, valid or malformed: a malformed one still
/// belongs to the object paste so it can report the error to the author.
fn is_our_payload(text: &str) -> bool {
    Clipboard::from_text(text).is_some() || text.contains(CLIPBOARD_KEY)
}

/// Decide whether Cmd/Ctrl+V pastes an image, ignoring a CourseCraft payload.
///
/// Takes plain clipboard state rather than a real [`arboard::Clipboard`] so the
/// decision is unit-testable on any platform.
#[must_use]
pub fn paste_image_action(text: Option<&str>, image: Option<&ImageData<'_>>) -> PasteImageAction {
    if text.is_some_and(is_our_payload) {
        return PasteImageAction::LeaveToObjectPaste;
    }
    if image.is_some() {
        return PasteImageAction::PasteImage;
    }
    PasteImageAction::DoNothing
}

/// Handle a Cmd/Ctrl+V pressed this frame, before the editor UI runs.
///
/// All clipboard and encoding failures are ignored quietly except a failed PNG
/// encode, which reports a status message. The web build never calls this.
pub fn maybe_paste_image(ctx: &egui::Context, editor: &mut Editor) {
    let v_pressed = ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::V));
    if !v_pressed || !editor.wants_image_paste(ctx) {
        return;
    }

    let Ok(mut clipboard) = arboard::Clipboard::new() else {
        return;
    };
    let text = clipboard.get_text().ok();
    let image = clipboard.get_image().ok();
    if paste_image_action(text.as_deref(), image.as_ref()) != PasteImageAction::PasteImage {
        return;
    }

    if let Some(image) = image {
        let dimensions = u32::try_from(image.width)
            .ok()
            .zip(u32::try_from(image.height).ok());
        let encoded = dimensions
            .and_then(|(width, height)| encode_png_rgba(width, height, image.bytes.as_ref()).ok());
        match encoded {
            Some(png) => {
                editor.paste_image_png(&png);
            }
            None => editor.set_status("Couldn't paste the image"),
        }
    }

    // The clipboard text must not be pasted as text on top of the new picture.
    ctx.input_mut(|i| {
        i.events
            .retain(|event| !matches!(event, egui::Event::Paste(_)))
    });
}

#[cfg(test)]
mod tests {
    use super::{CLIPBOARD_KEY, PasteImageAction, paste_image_action};
    use arboard::ImageData;

    fn image_data() -> ImageData<'static> {
        ImageData {
            width: 2,
            height: 1,
            bytes: std::borrow::Cow::Owned(vec![255, 0, 0, 255, 0, 255, 0, 255]),
        }
    }

    #[test]
    fn our_marker_leaves_the_paste_to_object_paste() {
        let text = format!("{{\"{CLIPBOARD_KEY}\":1,\"kind\":\"objects\"}}");
        assert_eq!(
            paste_image_action(Some(&text), Some(&image_data())),
            PasteImageAction::LeaveToObjectPaste
        );
    }

    #[test]
    fn an_image_with_other_text_is_pasted() {
        assert_eq!(
            paste_image_action(Some("a screenshot"), Some(&image_data())),
            PasteImageAction::PasteImage
        );
        assert_eq!(
            paste_image_action(None, Some(&image_data())),
            PasteImageAction::PasteImage
        );
    }

    #[test]
    fn no_image_does_nothing() {
        assert_eq!(
            paste_image_action(Some("plain text"), None),
            PasteImageAction::DoNothing
        );
        assert_eq!(paste_image_action(None, None), PasteImageAction::DoNothing);
    }
}
