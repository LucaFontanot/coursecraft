//! The ribbon as data: every tab, group and control in order (docs/ui/ribbon.md).
//!
//! The drawing code in `ribbon/mod.rs` walks these tables, so the docs and the UI
//! stay in step and a control whose feature is missing is drawn disabled with its
//! reason, never left out. Every label, caption and reason here is English source
//! text, looked up through `tl!` when drawn.

use coursecraft_render::Icon;

/// One tab of the strip.
pub struct Tab {
    /// Stable id, saved as the open tab.
    pub id: &'static str,
    pub label: &'static str,
    pub when: TabWhen,
    /// The groups, left to right. Empty while the tab is still being built.
    pub groups: &'static [Group],
    /// Groups shown instead while the Course Map is open (Home only).
    pub map_groups: Option<&'static [Group]>,
}

/// When a tab's controls can be used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabWhen {
    Always,
    /// Drawn always; its controls are disabled in the Course Map.
    SlideOpen,
}

/// A group of controls with its caption under it.
pub struct Group {
    pub id: &'static str,
    pub label: &'static str,
    /// The ↘ dialog launcher in the caption.
    pub launcher: Option<Action>,
    pub items: &'static [Item],
}

/// One control.
pub struct Item {
    /// Unique across the ribbon.
    pub id: &'static str,
    pub label: &'static str,
    pub icon: Icon,
    pub kind: Kind,
    pub action: Action,
    /// One sentence of help in the tooltip; empty for none.
    pub help: &'static str,
    /// The ▾ menu of a split control.
    pub menu: Menu,
}

/// How a control is drawn. Consecutive small kinds stack in columns of three.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// 32 px icon over the label.
    Large,
    /// Large, with a ▾ strip under the label for its menu.
    LargeSplit,
    /// 16 px icon beside the label.
    Small,
    /// Small, with a ▾ for its menu.
    SmallSplit,
    /// A small icon-only toggle (Bold, Align Left…); its label is the tooltip.
    Toggle,
    /// A text field with a ▾ (font family, size), this wide.
    Combo { width: f32 },
}

impl Kind {
    /// Drawn in a column of small controls.
    #[must_use]
    pub fn is_small(self) -> bool {
        !matches!(self, Kind::Large | Kind::LargeSplit)
    }

    /// Has a ▾ menu.
    #[must_use]
    pub fn is_split(self) -> bool {
        matches!(self, Kind::LargeSplit | Kind::SmallSplit)
    }
}

/// What a control does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// An engine command id, with parameters from the selection (`menu::adapter`).
    Command(&'static str),
    /// An editor-side action.
    Ui(UiAction),
    /// Drawn disabled: "Not available yet: {reason}".
    NotYet(&'static str),
    /// Drawn disabled: "Not planned: {reason}".
    NotPlanned(&'static str),
}

/// Editor-side actions (dialogs, views, the clipboard), run in `ribbon/mod.rs`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiAction {
    Paste,
    Cut,
    Copy,
    FormatPainter,
    NewSlide,
    NewScene,
    DuplicateSlide,
    NarrationFromText,
    FindReplace,
    Spelling,
    FormatInspector,
    Arrange,
    Player,
    Preview,
    Publish,
    // Insert
    SlideLayer,
    Picture,
    MediaLibrary,
    Shape,
    Video,
    Audio,
    RecordNarration,
    WebObject,
    Lottie,
    TextBox,
    Table,
    Button,
    Slider,
    Dial,
    InputText,
    InputCheckbox,
    InputRadio,
    Marker,
    ScrollPanel,
    Trigger,
    Interaction,
    // Slides
    Question,
    ResultsSlide,
    Certificate,
    ImportPptx,
    ImportQuestions,
    ImportDocument,
    ImportOutline,
    SlideMaster,
    // View
    CourseMap,
    SlideView,
    ZoomFit,
    ZoomIn,
    ZoomOut,
    Zoom100,
    Rulers,
    Grid,
    Guides,
    SnapToGrid,
    SnapToGuides,
    PanelSlides,
    PanelTimeline,
    PanelInspector,
    ResetPanels,
    ThemeLight,
    ThemeDark,
    ThemeSystem,
    // Design, Transitions, Animations
    SlideSize,
    Themes,
    SlideInspector,
    AnimateInspector,
    AddMotionPath,
    // Help
    Guide,
    Shortcuts,
    Welcome,
    GitHub,
    ReportProblem,
    SuggestFeature,
    CheckUpdates,
    ConnectAgent,
    AiSettings,
    Automation,
}

/// A split control's ▾ menu.
#[derive(Clone, Copy)]
pub enum Menu {
    None,
    /// Entries drawn from the spec, by the same rules as controls.
    Items(&'static [Item]),
    /// A menu the editor already has, reused as is.
    Ui(UiMenu),
}

/// Menus the editor already draws elsewhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiMenu {
    /// New Slide ▾ (`slides::show_new_slide_popup`).
    NewSlide,
    /// Preview ▾ (this slide, selected slides, from the start).
    Preview,
    /// Arrange ▾ (the Arrange menu's commands).
    Arrange,
    /// Shape ▾ (the shape gallery).
    Shapes,
    /// Question ▾ (the question kinds).
    Questions,
}

/// Why a control whose feature exists is not wired to the ribbon yet (M30 step 2).
pub const WIRING_FORMAT: &str = "ribbon wiring (use the Format inspector)";
const WIRING_SLIDE: &str = "ribbon wiring (use the Slide inspector)";
const WIRING_BANKS: &str = "ribbon wiring (use Question banks in the slides panel)";

const fn item(
    id: &'static str,
    label: &'static str,
    icon: Icon,
    kind: Kind,
    action: Action,
) -> Item {
    Item {
        id,
        label,
        icon,
        kind,
        action,
        help: "",
        menu: Menu::None,
    }
}

const fn split(
    id: &'static str,
    label: &'static str,
    icon: Icon,
    kind: Kind,
    action: Action,
    menu: Menu,
) -> Item {
    Item {
        id,
        label,
        icon,
        kind,
        action,
        help: "",
        menu,
    }
}

const fn helped(mut i: Item, help: &'static str) -> Item {
    i.help = help;
    i
}

use Action::{Command, NotPlanned, NotYet, Ui};
use Kind::{Combo, Large, LargeSplit, Small, SmallSplit, Toggle};

// ---------------------------------------------------------------------------
// Shared groups
// ---------------------------------------------------------------------------

const PASTE_MENU: &[Item] = &[
    item(
        "paste.destination",
        "Use Destination Theme",
        Icon::Notepad,
        Small,
        Ui(UiAction::Paste),
    ),
    item(
        "paste.keep_source",
        "Keep Source Formatting",
        Icon::Notepad,
        Small,
        NotYet("paste keeping the source formatting"),
    ),
    item(
        "paste.text_only",
        "Keep Text Only",
        Icon::TextT,
        Small,
        NotYet("paste as plain text"),
    ),
    item(
        "paste.special",
        "Paste Special…",
        Icon::Notepad,
        Small,
        NotYet("Paste Special"),
    ),
];

const COPY_MENU: &[Item] = &[
    item("copy.copy", "Copy", Icon::Copy, Small, Ui(UiAction::Copy)),
    item(
        "copy.duplicate",
        "Duplicate",
        Icon::Copy,
        Small,
        Command("object.duplicate"),
    ),
];

const PUBLISH: Group = Group {
    id: "publish",
    label: "Publish",
    launcher: None,
    items: &[
        helped(
            item(
                "publish.player",
                "Player",
                Icon::Monitor,
                Large,
                Ui(UiAction::Player),
            ),
            "Set up the player around the course.",
        ),
        helped(
            split(
                "publish.preview",
                "Preview",
                Icon::Play,
                LargeSplit,
                Ui(UiAction::Preview),
                Menu::Ui(UiMenu::Preview),
            ),
            "Play the current slide as a learner sees it.",
        ),
        helped(
            item(
                "publish.publish",
                "Publish",
                Icon::UploadSimple,
                Large,
                Ui(UiAction::Publish),
            ),
            "Publish the course for the web or an LMS.",
        ),
    ],
};

const NEW_SLIDE: Item = helped(
    split(
        "slide.new",
        "New Slide",
        Icon::PlusSquare,
        LargeSplit,
        Ui(UiAction::NewSlide),
        Menu::Ui(UiMenu::NewSlide),
    ),
    "Add a slide after the current one.",
);

// ---------------------------------------------------------------------------
// Home (Slide variant)
// ---------------------------------------------------------------------------

const HOME_SLIDE: &[Group] = &[
    Group {
        id: "clipboard",
        label: "Clipboard",
        launcher: None,
        items: &[
            split(
                "clipboard.paste",
                "Paste",
                Icon::Notepad,
                LargeSplit,
                Ui(UiAction::Paste),
                Menu::Items(PASTE_MENU),
            ),
            item(
                "clipboard.cut",
                "Cut",
                Icon::XSquare,
                Small,
                Ui(UiAction::Cut),
            ),
            split(
                "clipboard.copy",
                "Copy",
                Icon::Copy,
                SmallSplit,
                Ui(UiAction::Copy),
                Menu::Items(COPY_MENU),
            ),
            helped(
                item(
                    "clipboard.format_painter",
                    "Format Painter",
                    Icon::PaintBrush,
                    Small,
                    Ui(UiAction::FormatPainter),
                ),
                "Copy the selected object's format. Double-click to keep it on.",
            ),
        ],
    },
    Group {
        id: "slide",
        label: "Slide",
        launcher: None,
        items: &[
            NEW_SLIDE,
            item(
                "slide.apply_layout",
                "Apply Layout",
                Icon::Layout,
                SmallSplit,
                NotYet(WIRING_SLIDE),
            ),
            item(
                "slide.focus_order",
                "Focus Order",
                Icon::ListNumbers,
                Small,
                NotYet("the Focus Order dialog"),
            ),
            item(
                "slide.duplicate",
                "Duplicate",
                Icon::Copy,
                Small,
                Ui(UiAction::DuplicateSlide),
            ),
        ],
    },
    Group {
        id: "ai",
        label: "AI",
        launcher: None,
        items: &[
            item(
                "ai.insert_text",
                "Insert Text",
                Icon::Sparkle,
                Large,
                NotYet("AI text drafting"),
            ),
            item(
                "ai.edit_text",
                "Edit Text",
                Icon::PencilSimple,
                Large,
                NotYet("AI rewriting of selected text"),
            ),
            item(
                "ai.insert_image",
                "Insert Image",
                Icon::Image,
                Large,
                NotYet("AI images"),
            ),
            split(
                "ai.insert_audio",
                "Insert Audio",
                Icon::SpeakerHigh,
                LargeSplit,
                Ui(UiAction::NarrationFromText),
                Menu::Items(&[
                    item(
                        "ai.audio.narration",
                        "Narration from Text…",
                        Icon::SpeakerHigh,
                        Small,
                        Ui(UiAction::NarrationFromText),
                    ),
                    item(
                        "ai.audio.effect",
                        "Sound Effect",
                        Icon::SpeakerHigh,
                        Small,
                        NotYet("AI sound effects"),
                    ),
                    item(
                        "ai.audio.music",
                        "Music",
                        Icon::SpeakerHigh,
                        Small,
                        NotYet("AI music"),
                    ),
                ]),
            ),
            item(
                "ai.quiz",
                "Quiz",
                Icon::ListChecks,
                Small,
                NotYet("AI quiz drafting"),
            ),
            item(
                "ai.question",
                "Question",
                Icon::Question,
                Small,
                NotYet("AI question drafting"),
            ),
            item(
                "ai.summary",
                "Summary",
                Icon::Article,
                Small,
                NotYet("AI summaries"),
            ),
        ],
    },
    Group {
        id: "font",
        label: "Font",
        launcher: Some(NotYet("the Font dialog")),
        items: &[
            item(
                "font.text_styles",
                "Text Styles",
                Icon::TextAa,
                LargeSplit,
                NotYet(WIRING_FORMAT),
            ),
            // Three rows of five, in Storyline's reading order.
            item(
                "font.family",
                "Font",
                Icon::TextT,
                Combo { width: 128.0 },
                NotYet(WIRING_FORMAT),
            ),
            item(
                "font.size",
                "Font size",
                Icon::TextT,
                Combo { width: 52.0 },
                NotYet(WIRING_FORMAT),
            ),
            item(
                "font.grow",
                "Grow Font",
                Icon::CaretUp,
                Toggle,
                NotYet(WIRING_FORMAT),
            ),
            item(
                "font.shrink",
                "Shrink Font",
                Icon::CaretDown,
                Toggle,
                NotYet(WIRING_FORMAT),
            ),
            item(
                "font.clear",
                "Clear Formatting",
                Icon::X,
                Toggle,
                NotYet("clearing formatting"),
            ),
            item(
                "font.bold",
                "Bold",
                Icon::TextB,
                Toggle,
                NotYet(WIRING_FORMAT),
            ),
            item(
                "font.italic",
                "Italic",
                Icon::TextItalic,
                Toggle,
                NotYet(WIRING_FORMAT),
            ),
            item(
                "font.underline",
                "Underline",
                Icon::TextUnderline,
                Toggle,
                NotYet(WIRING_FORMAT),
            ),
            item(
                "font.strikethrough",
                "Strikethrough",
                Icon::Minus,
                Toggle,
                NotYet(WIRING_FORMAT),
            ),
            item(
                "font.shadow",
                "Text Shadow",
                Icon::TextT,
                Toggle,
                NotYet("text shadow"),
            ),
            item(
                "font.spacing",
                "Character Spacing",
                Icon::ArrowsOut,
                Toggle,
                NotYet(WIRING_FORMAT),
            ),
            item(
                "font.case",
                "Change Case",
                Icon::TextAa,
                Toggle,
                NotYet("changing case"),
            ),
            item(
                "font.highlight",
                "Highlight",
                Icon::PaintBrush,
                Toggle,
                NotYet("text highlight colour"),
            ),
            item(
                "font.color",
                "Font Color",
                Icon::Palette,
                Toggle,
                NotYet(WIRING_FORMAT),
            ),
            helped(
                item(
                    "font.spelling",
                    "Spelling",
                    Icon::CheckSquare,
                    Toggle,
                    Ui(UiAction::Spelling),
                ),
                "Check the spelling of the whole course.",
            ),
        ],
    },
    Group {
        id: "paragraph",
        label: "Paragraph",
        launcher: Some(NotYet("the Paragraph dialog")),
        items: &[
            item(
                "paragraph.bullets",
                "Bullets",
                Icon::ListBullets,
                Toggle,
                NotYet(WIRING_FORMAT),
            ),
            item(
                "paragraph.numbering",
                "Numbering",
                Icon::ListNumbers,
                Toggle,
                NotYet(WIRING_FORMAT),
            ),
            item(
                "paragraph.indent_less",
                "Decrease Indent",
                Icon::ArrowLeft,
                Toggle,
                NotYet(WIRING_FORMAT),
            ),
            item(
                "paragraph.indent_more",
                "Increase Indent",
                Icon::ArrowRight,
                Toggle,
                NotYet(WIRING_FORMAT),
            ),
            item(
                "paragraph.line_spacing",
                "Line Spacing",
                Icon::Rows,
                Toggle,
                NotYet(WIRING_FORMAT),
            ),
            item(
                "paragraph.align_left",
                "Align Left",
                Icon::TextAlignLeft,
                Toggle,
                NotYet(WIRING_FORMAT),
            ),
            item(
                "paragraph.align_center",
                "Center",
                Icon::TextAlignCenter,
                Toggle,
                NotYet(WIRING_FORMAT),
            ),
            item(
                "paragraph.align_right",
                "Align Right",
                Icon::TextAlignRight,
                Toggle,
                NotYet(WIRING_FORMAT),
            ),
            item(
                "paragraph.justify",
                "Justify",
                Icon::List,
                Toggle,
                NotYet(WIRING_FORMAT),
            ),
            item(
                "paragraph.direction",
                "Text Direction",
                Icon::ArrowsDownUp,
                SmallSplit,
                NotYet("vertical and rotated text"),
            ),
            item(
                "paragraph.align_text",
                "Align Text",
                Icon::TextColumns,
                SmallSplit,
                NotYet(WIRING_FORMAT),
            ),
            helped(
                item(
                    "paragraph.find",
                    "Find / Replace",
                    Icon::MagnifyingGlass,
                    Small,
                    Ui(UiAction::FindReplace),
                ),
                "Find text in the course and replace it.",
            ),
        ],
    },
    Group {
        id: "drawing",
        label: "Drawing",
        launcher: Some(Ui(UiAction::FormatInspector)),
        items: &[
            split(
                "drawing.arrange",
                "Arrange",
                Icon::Stack,
                LargeSplit,
                Ui(UiAction::Arrange),
                Menu::Ui(UiMenu::Arrange),
            ),
            item(
                "drawing.quick_styles",
                "Quick Styles",
                Icon::Palette,
                LargeSplit,
                NotYet(WIRING_FORMAT),
            ),
            item(
                "drawing.fill",
                "Shape Fill",
                Icon::PaintBrush,
                SmallSplit,
                NotYet(WIRING_FORMAT),
            ),
            item(
                "drawing.outline",
                "Shape Outline",
                Icon::Square,
                SmallSplit,
                NotYet(WIRING_FORMAT),
            ),
            item(
                "drawing.effect",
                "Shape Effect",
                Icon::Sparkle,
                SmallSplit,
                NotYet(WIRING_FORMAT),
            ),
        ],
    },
    PUBLISH,
];

// ---------------------------------------------------------------------------
// Home (Course Map variant)
// ---------------------------------------------------------------------------

const HOME_MAP: &[Group] = &[
    Group {
        id: "map.clipboard",
        label: "Clipboard",
        launcher: None,
        items: &[
            item(
                "map.paste",
                "Paste",
                Icon::Notepad,
                Large,
                Ui(UiAction::Paste),
            ),
            item("map.cut", "Cut", Icon::XSquare, Small, Ui(UiAction::Cut)),
            item("map.copy", "Copy", Icon::Copy, Small, Ui(UiAction::Copy)),
        ],
    },
    Group {
        id: "map.scenes",
        label: "Scenes",
        launcher: None,
        items: &[
            helped(
                item(
                    "map.new_scene",
                    "New Scene",
                    Icon::FilePlus,
                    Large,
                    Ui(UiAction::NewScene),
                ),
                "Add a scene with one slide.",
            ),
            item(
                "map.starting_scene",
                "Starting Scene",
                Icon::Flag,
                Small,
                NotYet("a starting scene other than the first"),
            ),
            item(
                "map.index",
                "Index",
                Icon::ListBullets,
                Small,
                NotYet("the scene index"),
            ),
            item(
                "map.question_banks",
                "Question Banks",
                Icon::Cards,
                SmallSplit,
                NotYet(WIRING_BANKS),
            ),
        ],
    },
    Group {
        id: "map.ai",
        label: "AI",
        launcher: None,
        items: &[
            item(
                "map.ai.quiz",
                "Quiz",
                Icon::ListChecks,
                Large,
                NotYet("AI quiz drafting"),
            ),
            item(
                "map.ai.question",
                "Question",
                Icon::Question,
                Large,
                NotYet("AI question drafting"),
            ),
            item(
                "map.ai.summary",
                "Summary",
                Icon::Article,
                Large,
                NotYet("AI summaries"),
            ),
        ],
    },
    Group {
        id: "map.slide",
        label: "Slide",
        launcher: None,
        items: &[
            Item {
                id: "map.new_slide",
                ..NEW_SLIDE
            },
            item(
                "map.apply_layout",
                "Apply Layout",
                Icon::Layout,
                SmallSplit,
                NotYet(WIRING_SLIDE),
            ),
            item(
                "map.reset",
                "Reset",
                Icon::ArrowCounterClockwise,
                Small,
                NotYet("resetting a slide to its layout"),
            ),
            item(
                "map.duplicate",
                "Duplicate",
                Icon::Copy,
                Small,
                Ui(UiAction::DuplicateSlide),
            ),
            item(
                "map.record_screen",
                "Record Screen",
                Icon::VideoCamera,
                Large,
                NotPlanned("screen recording"),
            ),
        ],
    },
    Group {
        id: "map.publish",
        ..PUBLISH
    },
];

// ---------------------------------------------------------------------------
// Insert
// ---------------------------------------------------------------------------

const PICTURE_MENU: &[Item] = &[
    item(
        "picture.file",
        "Picture from File…",
        Icon::Image,
        Small,
        Ui(UiAction::Picture),
    ),
    item(
        "picture.library",
        "Media Library…",
        Icon::FolderOpen,
        Small,
        Ui(UiAction::MediaLibrary),
    ),
    item(
        "picture.screenshot",
        "Screenshot",
        Icon::Monitor,
        Small,
        NotYet("screenshots"),
    ),
];

const VIDEO_MENU: &[Item] = &[
    item(
        "video.file",
        "Video from File…",
        Icon::FilmStrip,
        Small,
        Ui(UiAction::Video),
    ),
    item(
        "video.website",
        "Video from Website…",
        Icon::Link,
        Small,
        Ui(UiAction::WebObject),
    ),
    item(
        "video.webcam",
        "Record Webcam",
        Icon::VideoCamera,
        Small,
        NotYet("webcam recording"),
    ),
    item(
        "video.library",
        "Media Library…",
        Icon::FolderOpen,
        Small,
        Ui(UiAction::MediaLibrary),
    ),
];

const AUDIO_MENU: &[Item] = &[
    item(
        "audio.file",
        "Audio from File…",
        Icon::SpeakerHigh,
        Small,
        Ui(UiAction::Audio),
    ),
    item(
        "audio.library",
        "Media Library…",
        Icon::FolderOpen,
        Small,
        Ui(UiAction::MediaLibrary),
    ),
    item(
        "audio.record",
        "Record Mic…",
        Icon::Headphones,
        Small,
        Ui(UiAction::RecordNarration),
    ),
    item(
        "audio.tts",
        "Text-to-Speech…",
        Icon::SpeakerHigh,
        Small,
        Ui(UiAction::NarrationFromText),
    ),
    item(
        "audio.ai",
        "AI Audio",
        Icon::Sparkle,
        Small,
        NotYet("AI sound effects and music"),
    ),
    item(
        "audio.options",
        "Options…",
        Icon::Gear,
        Small,
        NotYet("the course audio options"),
    ),
];

const INPUT_MENU: &[Item] = &[
    item(
        "input.text",
        "Text Entry",
        Icon::TextT,
        Small,
        Ui(UiAction::InputText),
    ),
    item(
        "input.checkbox",
        "Checkbox",
        Icon::CheckSquare,
        Small,
        Ui(UiAction::InputCheckbox),
    ),
    item(
        "input.radio",
        "Radio Buttons",
        Icon::RadioButton,
        Small,
        Ui(UiAction::InputRadio),
    ),
];

const INSERT: &[Group] = &[
    Group {
        id: "insert.slide",
        label: "Slide",
        launcher: None,
        items: &[
            helped(
                item(
                    "insert.slide_layer",
                    "Slide Layer",
                    Icon::Stack,
                    Large,
                    Ui(UiAction::SlideLayer),
                ),
                "Add a layer to the current slide.",
            ),
            item(
                "insert.convert_question",
                "Convert to Custom Question",
                Icon::Question,
                Large,
                NotYet("turning a slide's objects into a custom question"),
            ),
            item(
                "insert.zoom_region",
                "Zoom Region",
                Icon::MagnifyingGlassPlus,
                Large,
                NotYet("zoom regions"),
            ),
        ],
    },
    Group {
        id: "insert.library",
        label: "Library",
        launcher: None,
        items: &[
            item(
                "insert.characters",
                "Characters",
                Icon::Robot,
                Large,
                NotPlanned("characters"),
            ),
            item(
                "insert.photos",
                "Photos",
                Icon::Image,
                Large,
                NotPlanned("a content library"),
            ),
            item(
                "insert.illustrations",
                "Illustrations",
                Icon::Palette,
                Small,
                NotPlanned("a content library"),
            ),
            item(
                "insert.icons",
                "Icons",
                Icon::Shapes,
                Small,
                NotYet("a bundled icon set"),
            ),
            item(
                "insert.videos",
                "Videos",
                Icon::FilmStrip,
                Small,
                NotPlanned("a content library"),
            ),
        ],
    },
    Group {
        id: "insert.media",
        label: "Media",
        launcher: None,
        items: &[
            helped(
                split(
                    "insert.picture",
                    "Picture",
                    Icon::Image,
                    LargeSplit,
                    Ui(UiAction::Picture),
                    Menu::Items(PICTURE_MENU),
                ),
                "Insert a picture from a file.",
            ),
            item(
                "insert.image_360",
                "360° Image",
                Icon::Circle,
                Large,
                NotPlanned("360° images"),
            ),
            helped(
                split(
                    "insert.shape",
                    "Shape",
                    Icon::Shapes,
                    LargeSplit,
                    Ui(UiAction::Shape),
                    Menu::Ui(UiMenu::Shapes),
                ),
                "Pick a shape, then drag on the slide.",
            ),
            item(
                "insert.caption",
                "Caption",
                Icon::Subtitles,
                LargeSplit,
                NotYet("callout shapes"),
            ),
            split(
                "insert.video",
                "Video",
                Icon::FilmStrip,
                LargeSplit,
                Ui(UiAction::Video),
                Menu::Items(VIDEO_MENU),
            ),
            split(
                "insert.audio",
                "Audio",
                Icon::SpeakerHigh,
                LargeSplit,
                Ui(UiAction::Audio),
                Menu::Items(AUDIO_MENU),
            ),
            helped(
                item(
                    "insert.web_object",
                    "Web Object",
                    Icon::Link,
                    Large,
                    Ui(UiAction::WebObject),
                ),
                "Embed a web page or a video from a website.",
            ),
            helped(
                item(
                    "insert.lottie",
                    "Animation",
                    Icon::Sparkle,
                    Large,
                    Ui(UiAction::Lottie),
                ),
                "Insert a Lottie animation.",
            ),
        ],
    },
    Group {
        id: "insert.text",
        label: "Text",
        launcher: None,
        items: &[
            helped(
                item(
                    "insert.text_box",
                    "Text Box",
                    Icon::TextT,
                    Large,
                    Ui(UiAction::TextBox),
                ),
                "Drag on the slide to place a text box.",
            ),
            item(
                "insert.table",
                "Table",
                Icon::Rows,
                Large,
                Ui(UiAction::Table),
            ),
            item(
                "insert.slide_number",
                "Slide Number",
                Icon::ListNumbers,
                Large,
                NotYet("slide number fields"),
            ),
            item(
                "insert.symbol",
                "Symbol",
                Icon::TextAa,
                Small,
                NotYet("the symbol picker"),
            ),
            item(
                "insert.reference",
                "Reference",
                Icon::BracketsCurly,
                Small,
                NotYet("inserting a variable reference"),
            ),
            item(
                "insert.hyperlink",
                "Hyperlink",
                Icon::Link,
                Small,
                NotYet(WIRING_FORMAT),
            ),
        ],
    },
    Group {
        id: "insert.interactive",
        label: "Interactive Objects",
        launcher: None,
        items: &[
            helped(
                item(
                    "insert.button",
                    "Button",
                    Icon::CursorClick,
                    Large,
                    Ui(UiAction::Button),
                ),
                "Drag on the slide to place a button.",
            ),
            item(
                "insert.slider",
                "Slider",
                Icon::SlidersHorizontal,
                Large,
                Ui(UiAction::Slider),
            ),
            item(
                "insert.dial",
                "Dial",
                Icon::Timer,
                Large,
                Ui(UiAction::Dial),
            ),
            item(
                "insert.hotspot",
                "Hotspot",
                Icon::Target,
                Large,
                NotYet("hotspot objects (a hotspot question has its own)"),
            ),
            split(
                "insert.input",
                "Input",
                Icon::TextColumns,
                LargeSplit,
                Ui(UiAction::InputText),
                Menu::Items(INPUT_MENU),
            ),
            item(
                "insert.marker",
                "Marker",
                Icon::Flag,
                Large,
                Ui(UiAction::Marker),
            ),
            helped(
                item(
                    "insert.trigger",
                    "Trigger",
                    Icon::Lightning,
                    Small,
                    Ui(UiAction::Trigger),
                ),
                "Add a trigger to the selected object or the slide.",
            ),
            item(
                "insert.scroll_panel",
                "Scrolling Panel",
                Icon::ArrowsDownUp,
                Small,
                Ui(UiAction::ScrollPanel),
            ),
            item(
                "insert.mouse",
                "Mouse",
                Icon::Cursor,
                Small,
                NotYet("a custom cursor per slide"),
            ),
            helped(
                item(
                    "insert.interaction",
                    "Interaction",
                    Icon::SquaresFour,
                    Large,
                    Ui(UiAction::Interaction),
                ),
                "Tabs, accordions, flip cards and more, built from layers and triggers.",
            ),
        ],
    },
    PUBLISH,
];

// ---------------------------------------------------------------------------
// Slides
// ---------------------------------------------------------------------------

const IMPORT_MENU: &[Item] = &[
    item(
        "import.pptx",
        "PowerPoint…",
        Icon::MicrosoftPowerpointLogo,
        Small,
        Ui(UiAction::ImportPptx),
    ),
    item(
        "import.questions",
        "Questions from CSV…",
        Icon::ListChecks,
        Small,
        Ui(UiAction::ImportQuestions),
    ),
    item(
        "import.document",
        "Course from Document…",
        Icon::Article,
        Small,
        Ui(UiAction::ImportDocument),
    ),
    item(
        "import.outline",
        "Slides from Outline…",
        Icon::ListBullets,
        Small,
        Ui(UiAction::ImportOutline),
    ),
];

const SLIDES: &[Group] = &[
    Group {
        id: "slides.slide",
        label: "Slide",
        launcher: None,
        items: &[
            Item {
                id: "slides.new_slide",
                ..NEW_SLIDE
            },
            split(
                "slides.import",
                "Import",
                Icon::FilePlus,
                LargeSplit,
                Ui(UiAction::ImportPptx),
                Menu::Items(IMPORT_MENU),
            ),
            item(
                "slides.duplicate",
                "Duplicate",
                Icon::Copy,
                Small,
                Ui(UiAction::DuplicateSlide),
            ),
        ],
    },
    Group {
        id: "slides.scene",
        label: "Scene",
        launcher: None,
        items: &[helped(
            item(
                "slides.new_scene",
                "New Scene",
                Icon::PlusSquare,
                Large,
                Ui(UiAction::NewScene),
            ),
            "Add a scene that starts with one slide.",
        )],
    },
    Group {
        id: "slides.questions",
        label: "Questions",
        launcher: None,
        items: &[
            helped(
                split(
                    "slides.question",
                    "Question",
                    Icon::Question,
                    LargeSplit,
                    Ui(UiAction::Question),
                    Menu::Ui(UiMenu::Questions),
                ),
                "Add a question slide: pick the kind.",
            ),
            item(
                "slides.banks",
                "Question Banks",
                Icon::Cards,
                LargeSplit,
                NotYet(WIRING_BANKS),
            ),
            item(
                "slides.results",
                "Results Slide",
                Icon::Trophy,
                Large,
                Ui(UiAction::ResultsSlide),
            ),
            item(
                "slides.certificate",
                "Certificate",
                Icon::GraduationCap,
                Large,
                Ui(UiAction::Certificate),
            ),
        ],
    },
    Group {
        id: "slides.masters",
        label: "Masters",
        launcher: None,
        items: &[
            item(
                "slides.slide_master",
                "Slide Master",
                Icon::Layout,
                Large,
                Ui(UiAction::SlideMaster),
            ),
            item(
                "slides.feedback_master",
                "Feedback Master",
                Icon::Layout,
                Large,
                NotYet("feedback masters"),
            ),
        ],
    },
    PUBLISH,
];

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

const VIEW: &[Group] = &[
    Group {
        id: "view.views",
        label: "Views",
        launcher: None,
        items: &[
            helped(
                item(
                    "view.course_map",
                    "Course Map",
                    Icon::SquaresFour,
                    Large,
                    Ui(UiAction::CourseMap),
                ),
                "Every scene and slide, with how they link.",
            ),
            item(
                "view.slide",
                "Slide",
                Icon::Presentation,
                Large,
                Ui(UiAction::SlideView),
            ),
            item(
                "view.slide_master",
                "Slide Master",
                Icon::Layout,
                Large,
                Ui(UiAction::SlideMaster),
            ),
            item(
                "view.feedback_master",
                "Feedback Master",
                Icon::Layout,
                Large,
                NotYet("feedback masters"),
            ),
        ],
    },
    Group {
        id: "view.zoom",
        label: "Zoom",
        launcher: None,
        items: &[
            item(
                "view.fit",
                "Fit in Window",
                Icon::CornersIn,
                Large,
                Ui(UiAction::ZoomFit),
            ),
            item(
                "view.zoom_in",
                "Zoom In",
                Icon::MagnifyingGlassPlus,
                Small,
                Ui(UiAction::ZoomIn),
            ),
            item(
                "view.zoom_out",
                "Zoom Out",
                Icon::MagnifyingGlassMinus,
                Small,
                Ui(UiAction::ZoomOut),
            ),
            item(
                "view.zoom_100",
                "100%",
                Icon::MagnifyingGlass,
                Small,
                Ui(UiAction::Zoom100),
            ),
        ],
    },
    Group {
        id: "view.show",
        label: "Show",
        launcher: None,
        items: &[
            item(
                "view.rulers",
                "Rulers",
                Icon::Rows,
                Small,
                Ui(UiAction::Rulers),
            ),
            item(
                "view.grid",
                "Grid",
                Icon::SquaresFour,
                Small,
                Ui(UiAction::Grid),
            ),
            item(
                "view.guides",
                "Guides",
                Icon::Plus,
                Small,
                Ui(UiAction::Guides),
            ),
            item(
                "view.snap_grid",
                "Snap to Grid",
                Icon::Selection,
                Small,
                Ui(UiAction::SnapToGrid),
            ),
            item(
                "view.snap_guides",
                "Snap to Guides",
                Icon::SelectionPlus,
                Small,
                Ui(UiAction::SnapToGuides),
            ),
        ],
    },
    Group {
        id: "view.panels",
        label: "Panels",
        launcher: None,
        items: &[
            item(
                "view.panel_slides",
                "Slides",
                Icon::SidebarSimple,
                Small,
                Ui(UiAction::PanelSlides),
            ),
            item(
                "view.panel_timeline",
                "Timeline",
                Icon::Clock,
                Small,
                Ui(UiAction::PanelTimeline),
            ),
            item(
                "view.panel_inspector",
                "Inspector",
                Icon::SlidersHorizontal,
                Small,
                Ui(UiAction::PanelInspector),
            ),
            helped(
                item(
                    "view.reset_panels",
                    "Reset Panels",
                    Icon::ArrowCounterClockwise,
                    Small,
                    Ui(UiAction::ResetPanels),
                ),
                "Put every panel back where it starts.",
            ),
        ],
    },
    Group {
        id: "view.theme",
        label: "Theme",
        launcher: None,
        items: &[
            item(
                "view.light",
                "Light",
                Icon::Sun,
                Small,
                Ui(UiAction::ThemeLight),
            ),
            item(
                "view.dark",
                "Dark",
                Icon::Moon,
                Small,
                Ui(UiAction::ThemeDark),
            ),
            item(
                "view.system",
                "Match System",
                Icon::Monitor,
                Small,
                Ui(UiAction::ThemeSystem),
            ),
        ],
    },
    PUBLISH,
];

// ---------------------------------------------------------------------------
// Design
// ---------------------------------------------------------------------------

const DESIGN: &[Group] = &[
    Group {
        id: "design.setup",
        label: "Setup",
        launcher: None,
        items: &[helped(
            item(
                "design.slide_size",
                "Slide Size",
                Icon::FrameCorners,
                Large,
                Ui(UiAction::SlideSize),
            ),
            "Change the size of every slide.",
        )],
    },
    Group {
        id: "design.themes",
        label: "Themes",
        launcher: None,
        items: &[helped(
            item(
                "design.themes",
                "Themes",
                Icon::Palette,
                Large,
                Ui(UiAction::Themes),
            ),
            "Pick a theme: colors and fonts for the whole course.",
        )],
    },
    Group {
        id: "design.options",
        label: "Theme Options",
        launcher: None,
        items: &[
            item(
                "design.colors",
                "Colors",
                Icon::Drop,
                Small,
                Ui(UiAction::Themes),
            ),
            item(
                "design.fonts",
                "Fonts",
                Icon::TextAa,
                Small,
                Ui(UiAction::Themes),
            ),
        ],
    },
    Group {
        id: "design.background",
        label: "Background",
        launcher: Some(Ui(UiAction::SlideInspector)),
        items: &[
            item(
                "design.background_styles",
                "Background Styles",
                Icon::Image,
                SmallSplit,
                NotYet(WIRING_SLIDE),
            ),
            item(
                "design.hide_background",
                "Hide Background Graphics",
                Icon::EyeSlash,
                Small,
                NotYet("hiding the master's graphics on one slide"),
            ),
        ],
    },
    PUBLISH,
];

// ---------------------------------------------------------------------------
// Transitions
// ---------------------------------------------------------------------------

const TRANSITIONS: &[Group] = &[
    Group {
        id: "transitions.to_this_slide",
        label: "Transition to This Slide",
        launcher: None,
        items: &[
            helped(
                item(
                    "transitions.pick",
                    "Transition",
                    Icon::FilmStrip,
                    Large,
                    Ui(UiAction::SlideInspector),
                ),
                "Pick the transition in the Slide inspector.",
            ),
            item(
                "transitions.options",
                "Effect Options",
                Icon::SlidersHorizontal,
                LargeSplit,
                NotYet(WIRING_SLIDE),
            ),
        ],
    },
    Group {
        id: "transitions.timing",
        label: "Timing",
        launcher: None,
        items: &[
            item(
                "transitions.duration",
                "Duration",
                Icon::Timer,
                Small,
                NotYet(WIRING_SLIDE),
            ),
            item(
                "transitions.apply_all",
                "Apply To All",
                Icon::Stack,
                Small,
                NotYet("applying one transition to every slide"),
            ),
        ],
    },
    PUBLISH,
];

// ---------------------------------------------------------------------------
// Animations
// ---------------------------------------------------------------------------

const ANIMATIONS: &[Group] = &[
    Group {
        id: "animations.advanced",
        label: "Advanced",
        launcher: None,
        items: &[item(
            "animations.painter",
            "Animation Painter",
            Icon::PaintBrush,
            Large,
            NotYet("the Animation Painter"),
        )],
    },
    Group {
        id: "animations.entrance",
        label: "Entrance Animations",
        launcher: None,
        items: &[helped(
            item(
                "animations.entrance",
                "Entrance",
                Icon::ArrowRight,
                Large,
                Ui(UiAction::AnimateInspector),
            ),
            "Pick how the object comes in, in the Animate inspector.",
        )],
    },
    Group {
        id: "animations.exit",
        label: "Exit Animations",
        launcher: None,
        items: &[helped(
            item(
                "animations.exit",
                "Exit",
                Icon::ArrowLeft,
                Large,
                Ui(UiAction::AnimateInspector),
            ),
            "Pick how the object goes out, in the Animate inspector.",
        )],
    },
    Group {
        id: "animations.emphasis",
        label: "Emphasis Animations",
        launcher: None,
        items: &[item(
            "animations.emphasis",
            "Add Emphasis Animation",
            Icon::Sparkle,
            LargeSplit,
            NotYet("emphasis animations"),
        )],
    },
    Group {
        id: "animations.motion",
        label: "Motion Paths",
        launcher: None,
        items: &[helped(
            item(
                "animations.motion_path",
                "Add Motion Path",
                Icon::Path,
                Large,
                Ui(UiAction::AddMotionPath),
            ),
            "Move the selected object along a path.",
        )],
    },
    Group {
        id: "animations.size",
        label: "Size and Position",
        launcher: Some(Ui(UiAction::FormatInspector)),
        items: &[item(
            "animations.size_position",
            "Size and Position",
            Icon::CornersOut,
            Large,
            Ui(UiAction::FormatInspector),
        )],
    },
    PUBLISH,
];

// ---------------------------------------------------------------------------
// Help
// ---------------------------------------------------------------------------

const HELP: &[Group] = &[
    Group {
        id: "help.help",
        label: "Help",
        launcher: None,
        items: &[
            item(
                "help.guide",
                "Guide",
                Icon::BookOpen,
                Large,
                Ui(UiAction::Guide),
            ),
            item(
                "help.shortcuts",
                "Keyboard Shortcuts",
                Icon::Keyboard,
                Large,
                Ui(UiAction::Shortcuts),
            ),
            item(
                "help.welcome",
                "Welcome Screen",
                Icon::House,
                Large,
                Ui(UiAction::Welcome),
            ),
        ],
    },
    Group {
        id: "help.community",
        label: "Community",
        launcher: None,
        items: &[
            item(
                "help.github",
                "CourseCraft on GitHub",
                Icon::ArrowSquareOut,
                Small,
                Ui(UiAction::GitHub),
            ),
            item(
                "help.report",
                "Report a Problem…",
                Icon::Warning,
                Small,
                Ui(UiAction::ReportProblem),
            ),
            item(
                "help.suggest",
                "Suggest a Feature…",
                Icon::Lightning,
                Small,
                Ui(UiAction::SuggestFeature),
            ),
        ],
    },
    Group {
        id: "help.updates",
        label: "Updates",
        launcher: None,
        items: &[item(
            "help.check_updates",
            "Check for Updates",
            Icon::ArrowClockwise,
            Large,
            Ui(UiAction::CheckUpdates),
        )],
    },
    Group {
        id: "help.ai",
        label: "AI",
        launcher: None,
        items: &[
            item(
                "help.connect_agent",
                "Connect an AI Agent…",
                Icon::Robot,
                Small,
                Ui(UiAction::ConnectAgent),
            ),
            item(
                "help.ai_settings",
                "AI Settings…",
                Icon::Gear,
                Small,
                Ui(UiAction::AiSettings),
            ),
            item(
                "help.automation",
                "Allow Automation",
                Icon::Lock,
                Small,
                Ui(UiAction::Automation),
            ),
        ],
    },
    Group {
        id: "help.about",
        label: "About",
        launcher: None,
        items: &[item(
            "help.about",
            "About CourseCraft",
            Icon::Info,
            Large,
            NotYet("an About window"),
        )],
    },
];

// ---------------------------------------------------------------------------
// The strip
// ---------------------------------------------------------------------------

/// The tab strip after File, in order (ribbon.md › Tab strip).
pub const TABS: &[Tab] = &[
    Tab {
        id: "home",
        label: "Home",
        when: TabWhen::Always,
        groups: HOME_SLIDE,
        map_groups: Some(HOME_MAP),
    },
    Tab {
        id: "insert",
        label: "Insert",
        when: TabWhen::SlideOpen,
        groups: INSERT,
        map_groups: None,
    },
    Tab {
        id: "slides",
        label: "Slides",
        when: TabWhen::Always,
        groups: SLIDES,
        map_groups: None,
    },
    Tab {
        id: "design",
        label: "Design",
        when: TabWhen::Always,
        groups: DESIGN,
        map_groups: None,
    },
    Tab {
        id: "transitions",
        label: "Transitions",
        when: TabWhen::SlideOpen,
        groups: TRANSITIONS,
        map_groups: None,
    },
    Tab {
        id: "animations",
        label: "Animations",
        when: TabWhen::SlideOpen,
        groups: ANIMATIONS,
        map_groups: None,
    },
    Tab {
        id: "view",
        label: "View",
        when: TabWhen::Always,
        groups: VIEW,
        map_groups: None,
    },
    Tab {
        id: "help",
        label: "Help",
        when: TabWhen::Always,
        groups: HELP,
        map_groups: None,
    },
];

/// The tab with this id.
#[must_use]
pub fn tab(id: &str) -> Option<&'static Tab> {
    TABS.iter().find(|t| t.id == id)
}

/// Every item of every tab, menus included, for checks.
pub fn all_items() -> impl Iterator<Item = &'static Item> {
    fn with_menu(item: &'static Item) -> Vec<&'static Item> {
        let mut out = vec![item];
        if let Menu::Items(entries) = item.menu {
            out.extend(entries.iter());
        }
        out
    }
    TABS.iter()
        .flat_map(|t| t.groups.iter().chain(t.map_groups.unwrap_or(&[]).iter()))
        .flat_map(|g| g.items.iter())
        .flat_map(with_menu)
}
