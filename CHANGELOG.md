# Changelog

What changed in each release, newest first. Versions follow [semantic versioning](https://semver.org): until 1.0, a minor version (0.x) may change things, but a project saved by an older CourseCraft always opens in a newer one.

## Unreleased

- A new sample course, **Healthy Habits at Work**, opens behind the welcome screen on a fresh launch and is first under Explore an example. Its 36 slides show off every feature: narration with objects timed to the spoken words (also inside layers), a video with cue points, a Lottie animation, all 12 question types, a question bank with a random draw, every ready-made interaction, inputs, timers, drag and drop, layers, states, a slide template and a certificate.
- Animations work again: what you pick in the Animations panel is now saved (it was lost, so nothing played and moving the object on the timeline seemed to reset it), and the timeline's Play button shows entrances and exits on the slide.
- A sound longer than its slide plays again from the start on the next Play, and scrubbing back works (it used to jump straight to the end).
- Deleting a button (or any object) that has triggers now works: its triggers go with it, the status bar says how many, and one Undo brings everything back. Deleting a layer takes the triggers that show it.
- Speaker notes typed in the Slide tab are saved (they were lost).
- Audio and video you place play by themselves, like narration, and a slide shorter than the sound or video grows to fit it.
- F12 previews the current slide and Shift+F12 previews from the start.
- Choosing a value that is already set no longer adds an Undo step that does nothing.
- Each scene in the slide list has a + button for a new slide in that scene, and New Scene starts with a slide of its own.
- Typing in long text with spell check on is fast again (it took most of a second per key in 0.2.0), the editor no longer redraws while idle, decoded pictures have a memory ceiling, and the slide list only draws the thumbnails you can see.
- Flip cards now have readable text on both sides with any theme colour (a white card with an accent outline, turning to an accent tint), and process-step labels are centred under their circles.

## 0.2.0

Everyday authoring: fonts, pictures, colours, spelling and ready-made interactions.

- **Your own fonts:** use any font installed on your computer. It is copied into the course so it publishes with it, with a note when the font's maker restricts embedding.
- **Paste pictures:** Cmd/Ctrl+V pastes a screenshot straight onto the slide. Copy and paste now also works with other apps.
- **Eyedropper:** pick any colour from the screen in every colour picker.
- **Spell check:** misspelled words are underlined as you type, with suggestions on right-click. Edit ▸ Check Spelling (F7) checks the whole course, and words you add are remembered.
- **Interactions gallery:** Insert ▸ Interaction shows ready-made, editable interactions, including six new ones: flip cards, hotspot reveal, process steps, timeline, reveal grid and scenario choice.

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
