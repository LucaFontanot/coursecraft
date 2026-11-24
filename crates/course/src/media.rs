//! Canonical read-only media reference traversal for a [`Course`], [`Slide`],
//! or [`Object`].
//!
//! All media-bearing model fields live in one place here so callers
//! (`course::media_references`, publishing, the editor, and the player) share a
//! single authoritative walk and never drift when new media fields are added.

use crate::{
    Course, FillStyle, Layer, Object, ObjectKind, QuestionBank, ResourceTarget, Slide,
    StateExtraKind, WebSource,
};

/// Which course containers [`visit_course_media`] traverses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaScope {
    /// Publishable containers only: course-level media, scenes, masters, and
    /// banks. Templates are authoring-only and excluded.
    Publishable,
    /// All containers, including authoring-only templates.
    AllIncludingTemplates,
}

/// The model slot that references a media path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaRole {
    Image,
    PictureFill,
    StateImage,
    StateExtraImage,
    VideoPoster,
    Lottie,
    Audio,
    Video,
    Caption,
    WebFolder,
    Font,
    BackgroundAudio,
    PlayerLogo,
    ResourceFile,
}

impl MediaRole {
    /// Whether the web player prefetches this role as a still image texture.
    #[must_use]
    pub const fn is_player_image(self) -> bool {
        matches!(
            self,
            Self::Image
                | Self::PictureFill
                | Self::StateImage
                | Self::StateExtraImage
                | Self::VideoPoster
        )
    }

    /// Whether the web player fetches this role through its in-memory LRU
    /// media cache (images plus Lottie JSON and caption tracks; excludes
    /// streamed audio/video sources and iframe web folders).
    #[must_use]
    pub const fn is_player_fetched(self) -> bool {
        self.is_player_image() || matches!(self, Self::Lottie | Self::Caption)
    }
}

/// Where a visited slide lives inside its course.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SlideContainer<'a> {
    /// A playable slide in a scene, with its 1-based play-order number.
    SceneSlide { number: usize },
    /// A slide master.
    Master,
    /// An authoring-only slide template.
    Template,
    /// A question slide inside a question bank.
    Bank { bank: &'a QuestionBank },
    /// A slide or object visited standalone outside a course walk.
    Detached,
}

/// The owning model entity of a [`MediaRef`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MediaOwner<'a> {
    Font {
        family: &'a str,
    },
    BackgroundAudio,
    PlayerLogo,
    Resource {
        id: &'a str,
        title: &'a str,
    },
    Object {
        container: SlideContainer<'a>,
        slide: Option<&'a Slide>,
        layer: Option<&'a Layer>,
        object: &'a Object,
        state_name: Option<&'a str>,
    },
}

/// A single media reference discovered by [`visit_course_media`],
/// [`visit_slide_media`], or [`visit_object_media`].
///
/// Populated slots with an empty `src` (`""`) are still emitted so callers
/// with strict path validation (such as publishing) can report them; callers
/// that ignore empty slots should filter `!r.src.is_empty()`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MediaRef<'a> {
    pub src: &'a str,
    pub role: MediaRole,
    pub owner: MediaOwner<'a>,
    /// Entry file name inside a web folder; present only when
    /// `role == MediaRole::WebFolder`.
    pub web_entry: Option<&'a str>,
}

/// Visit every media reference in `course` in canonical order:
/// `fonts`, `background_audio`, `player.logo`, file `resources`, `scenes`
/// (play order, 1-based), `masters`, `templates` (only for
/// [`MediaScope::AllIncludingTemplates`]), and `banks`.
pub fn visit_course_media<'a>(
    course: &'a Course,
    scope: MediaScope,
    visit: &mut dyn FnMut(MediaRef<'a>),
) {
    for face in &course.fonts {
        visit(MediaRef {
            src: &face.src,
            role: MediaRole::Font,
            owner: MediaOwner::Font {
                family: &face.family,
            },
            web_entry: None,
        });
    }
    if let Some(audio) = &course.background_audio {
        visit(MediaRef {
            src: &audio.src,
            role: MediaRole::BackgroundAudio,
            owner: MediaOwner::BackgroundAudio,
            web_entry: None,
        });
    }
    if let Some(logo) = &course.player.logo {
        visit(MediaRef {
            src: logo,
            role: MediaRole::PlayerLogo,
            owner: MediaOwner::PlayerLogo,
            web_entry: None,
        });
    }
    for resource in &course.resources {
        if let ResourceTarget::File { src } = &resource.target {
            visit(MediaRef {
                src,
                role: MediaRole::ResourceFile,
                owner: MediaOwner::Resource {
                    id: &resource.id,
                    title: &resource.title,
                },
                web_entry: None,
            });
        }
    }
    let mut number = 0usize;
    for scene in &course.scenes {
        for slide in &scene.slides {
            number = number.saturating_add(1);
            visit_slide_with_container(slide, SlideContainer::SceneSlide { number }, visit);
        }
    }
    for master in &course.masters {
        visit_slide_with_container(master, SlideContainer::Master, visit);
    }
    if scope == MediaScope::AllIncludingTemplates {
        for template in &course.templates {
            visit_slide_with_container(template, SlideContainer::Template, visit);
        }
    }
    for bank in &course.banks {
        for slide in &bank.slides {
            visit_slide_with_container(slide, SlideContainer::Bank { bank }, visit);
        }
    }
}

/// Visit every media reference on a single `slide` (base objects, then each
/// layer's objects) with [`SlideContainer::Detached`] and `slide: Some(slide)`.
pub fn visit_slide_media<'a>(slide: &'a Slide, visit: &mut dyn FnMut(MediaRef<'a>)) {
    visit_slide_with_container(slide, SlideContainer::Detached, visit);
}

/// Visit every media reference on a single `object` (its kind, states, and
/// state extras) with [`SlideContainer::Detached`] and `slide: None`.
pub fn visit_object_media<'a>(object: &'a Object, visit: &mut dyn FnMut(MediaRef<'a>)) {
    visit_object_with_owner(object, SlideContainer::Detached, None, None, visit);
}

fn visit_slide_with_container<'a>(
    slide: &'a Slide,
    container: SlideContainer<'a>,
    visit: &mut dyn FnMut(MediaRef<'a>),
) {
    for object in &slide.objects {
        visit_object_with_owner(object, container, Some(slide), None, visit);
    }
    for layer in &slide.layers {
        for object in &layer.objects {
            visit_object_with_owner(object, container, Some(slide), Some(layer), visit);
        }
    }
}

fn visit_object_with_owner<'a>(
    object: &'a Object,
    container: SlideContainer<'a>,
    slide: Option<&'a Slide>,
    layer: Option<&'a Layer>,
    visit: &mut dyn FnMut(MediaRef<'a>),
) {
    let base_owner = MediaOwner::Object {
        container,
        slide,
        layer,
        object,
        state_name: None,
    };
    match &object.kind {
        ObjectKind::Image { src, .. } => {
            visit(MediaRef {
                src,
                role: MediaRole::Image,
                owner: base_owner,
                web_entry: None,
            });
        }
        ObjectKind::Lottie { src, .. } => {
            visit(MediaRef {
                src,
                role: MediaRole::Lottie,
                owner: base_owner,
                web_entry: None,
            });
        }
        ObjectKind::Audio { src, captions, .. } => {
            visit(MediaRef {
                src,
                role: MediaRole::Audio,
                owner: base_owner,
                web_entry: None,
            });
            if let Some(captions) = captions {
                visit(MediaRef {
                    src: captions,
                    role: MediaRole::Caption,
                    owner: base_owner,
                    web_entry: None,
                });
            }
        }
        ObjectKind::Video {
            src,
            captions,
            poster,
            ..
        } => {
            visit(MediaRef {
                src,
                role: MediaRole::Video,
                owner: base_owner,
                web_entry: None,
            });
            if let Some(captions) = captions {
                visit(MediaRef {
                    src: captions,
                    role: MediaRole::Caption,
                    owner: base_owner,
                    web_entry: None,
                });
            }
            if let Some(poster) = poster {
                visit(MediaRef {
                    src: poster,
                    role: MediaRole::VideoPoster,
                    owner: base_owner,
                    web_entry: None,
                });
            }
        }
        ObjectKind::Shape { fill_style, .. } | ObjectKind::Button { fill_style, .. } => {
            if let Some(src) = fill_style.as_ref().and_then(FillStyle::picture_src) {
                visit(MediaRef {
                    src,
                    role: MediaRole::PictureFill,
                    owner: base_owner,
                    web_entry: None,
                });
            }
        }
        ObjectKind::Path { path } => {
            if let Some(src) = path.fill_style.as_ref().and_then(FillStyle::picture_src) {
                visit(MediaRef {
                    src,
                    role: MediaRole::PictureFill,
                    owner: base_owner,
                    web_entry: None,
                });
            }
        }
        ObjectKind::Web {
            source: WebSource::Folder { path, entry },
        } => {
            visit(MediaRef {
                src: path,
                role: MediaRole::WebFolder,
                owner: base_owner,
                web_entry: Some(entry),
            });
        }
        _ => {}
    }

    for state in &object.states {
        let state_owner = MediaOwner::Object {
            container,
            slide,
            layer,
            object,
            state_name: Some(state.name.as_str()),
        };
        if let Some(src) = &state.src {
            visit(MediaRef {
                src,
                role: MediaRole::StateImage,
                owner: state_owner,
                web_entry: None,
            });
        }
        if let Some(src) = state.fill_style.as_ref().and_then(FillStyle::picture_src) {
            visit(MediaRef {
                src,
                role: MediaRole::PictureFill,
                owner: state_owner,
                web_entry: None,
            });
        }
        for extra in &state.extras {
            let extra_src = match &extra.kind {
                StateExtraKind::Image { src, .. } => Some(src.as_str()),
                StateExtraKind::Shape { fill_style, .. } => {
                    fill_style.as_ref().and_then(FillStyle::picture_src)
                }
                StateExtraKind::Path { path } => {
                    path.fill_style.as_ref().and_then(FillStyle::picture_src)
                }
                StateExtraKind::Text { .. } => None,
            };
            if let Some(src) = extra_src {
                visit(MediaRef {
                    src,
                    role: MediaRole::StateExtraImage,
                    owner: state_owner,
                    web_entry: None,
                });
            }
        }
    }
}
