# Changelog

What changed in each release, newest first. Versions follow [semantic versioning](https://semver.org): until 1.0, a minor version (0.x) may change things, but a project saved by an older CourseCraft always opens in a newer one.

## Unreleased

- Use any font installed on your computer: the font picker lists them under "On this computer", and picking one copies it into the course so it publishes with it. Fonts whose maker restricts embedding show a note, and the Fonts window shows each font's licence text.
- Paste a screenshot from the system clipboard with Cmd/Ctrl+V: it lands in the middle of the slide, shrinks to fit if it is larger, and is selected, so one undo removes it. Copy and paste of text and objects now reaches other apps on the desktop too.
- Every colour picker has an Eyedropper button: it turns the cursor into a crosshair, shows a small swatch with the colour under the pointer, and clicking applies that colour in one undo step. Press Esc to cancel without changing anything.
- Misspelled words get a red wavy underline while editing. Right-click one for suggestions, "Add to Dictionary" or "Ignore". Edit ▸ Check Spelling (F7) walks every string in the course with Change, Change All, Ignore and Add to Dictionary; added words are remembered in Preferences.
- Insert ▸ Interaction opens a gallery of ready-made, editable interactions. Alongside tabs, accordion, markers, scrolling panels, timers and progress meters, there are flip cards that turn over when clicked, hotspots that reveal one callout at a time, and process steps driven by buttons and a variable.

## 0.1.2

- CourseCraft has an app icon: a "C" around a play button, in the ArtCraft family style. It shows in the Mac Dock and Finder, on the Windows .exe and taskbar, and on the Linux window.

## 0.1.1

The first release with downloads. The v0.1.0 tag never finished building, so its fixes are included here too.

**Cleaner layout**
- Dialogs open centred below the toolbar, stay on screen, and scroll when the window is small, with their action buttons always visible.
- Form fields in dialogs line up, and labels in the properties panel show in full.
- Fixed controls running past the properties panel's edge, timeline names running into their icons, and other overlaps and margins.
- An automatic check now catches overlapping or cut-off text, off-screen dialogs and clipped buttons at three window sizes, in dark and light.

**Builds**
- The Mac download is built on a current macOS build machine.

## 0.1.0 (first public release)

The first public release. CourseCraft is early: it is tested with automated tests and local mock servers, not yet certified against commercial LMSs. Try a package in your LMS before relying on it.

**Build courses**
- A visual editor for slides, scenes, layers, states, triggers, variables, timelines and animation, on macOS (Apple Silicon), Windows and Linux (less tested), and in the browser.
- Text, shapes, images, audio, video, web objects, timers, sliders, dials, scrolling panels, markers, progress meters, Lottie animations and motion paths.
- Slide masters, layouts, templates, format painter, slide size, named text styles and a theme palette.
- PowerPoint import, Word and PDF review exports, MP4 export, and XLIFF translation.
- Quizzes: twelve question types, question banks, results slides, retry and review. Questions can be imported from a CSV file.

**Learner player**
- Navigation modes, playback speed, full screen, captions, transcripts, glossary and resources, and keyboard and screen-reader use.

**Publish**
- SCORM 1.2, SCORM 2004 4th edition, cmi5, xAPI and LTI 1.3, with validation and accessibility checks first.

**Your files**
- Safe saving, version history, and project folders that work with git.

**Automation and AI**
- Every editing action is a command you can run from the command line, scripts, batch jobs or an AI agent (MCP). Optional AI that you bring yourself: off by default, never required.

**Feedback**
- Help → Report a Problem opens a pre-filled GitHub issue in your browser. Nothing is sent automatically.
