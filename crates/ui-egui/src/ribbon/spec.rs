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
}

/// Why a control whose feature exists is not wired to the ribbon yet (M30 step 2).
pub const WIRING_FORMAT: &str = "ribbon wiring (use the Format inspector)";
const WIRING_SLIDE: &str = "ribbon wiring (use the Slide inspector)";
const WIRING_BANKS: &str = "ribbon wiring (use Insert › Question Bank)";

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
        groups: &[],
        map_groups: None,
    },
    Tab {
        id: "slides",
        label: "Slides",
        when: TabWhen::Always,
        groups: &[],
        map_groups: None,
    },
    Tab {
        id: "design",
        label: "Design",
        when: TabWhen::Always,
        groups: &[],
        map_groups: None,
    },
    Tab {
        id: "transitions",
        label: "Transitions",
        when: TabWhen::SlideOpen,
        groups: &[],
        map_groups: None,
    },
    Tab {
        id: "animations",
        label: "Animations",
        when: TabWhen::SlideOpen,
        groups: &[],
        map_groups: None,
    },
    Tab {
        id: "view",
        label: "View",
        when: TabWhen::Always,
        groups: &[],
        map_groups: None,
    },
    Tab {
        id: "help",
        label: "Help",
        when: TabWhen::Always,
        groups: &[],
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
