# Changelog

What changed in each release, newest first. Versions follow [semantic versioning](https://semver.org): until 1.0, a minor version (0.x) may change things, but a project saved by an older CourseCraft always opens in a newer one.

## 0.2.1

A new sample course that shows off every feature, and the fixes we found by building it.

- **Healthy Habits at Work:** a new sample course opens behind the welcome screen on a fresh launch and is first under Explore an example. Its 36 slides use every feature: narration with objects timed to the spoken words (also inside layers), a video with cue points, a Lottie animation, all 12 question types, a question bank with a random draw, every ready-made interaction, layers, states, a slide template, a custom font, background music and a certificate.
- **Animations work again:** what you pick in the Animations panel is now saved, so entrances and exits play, and the timeline's Play button shows them on the slide.
- **Audio and video play by themselves:** media you place starts with the slide, like narration, and a slide shorter than its sound or video grows to fit. A long sound starts from the beginning on the next Play, and scrubbing back works.
- **Faster typing:** typing in long text with spell check on is fast again (it took most of a second per key in 0.2.0). The editor also stops redrawing while idle, and the slide list draws only the thumbnails you can see.
- **Quick preview:** F12 previews the current slide and Shift+F12 previews from the start.
- **Scenes:** each scene in the slide list has a + button for a new slide in that scene, and New Scene starts with a slide of its own.

**Fixes**
- Deleting an object that has triggers works: its triggers go with it, the status bar says how many, and one Undo brings everything back. Deleting a layer also removes the triggers that show it.
- Speaker notes typed in the Slide tab are saved.
- Saving keeps the course's background music and any media used only by question-bank slides.
- The player's outline drawer no longer draws scrolled rows over its header and tabs, and its rows ignore clicks while it slides in. On the web, the screen-reader layer no longer drifts away from the rows it describes.
- Choosing a value that is already set no longer adds an Undo step that does nothing.
- Flip cards have readable text on both sides with any theme colour, and process-step labels are centred under their circles.
- A click on a menu or popup no longer also clicks what lies underneath. Picking "Radial gradient" from the Fill type menu in a smaller window used to select the slide layer under the menu too: the layer's content appeared over the slide and the panel jumped to the Slide tab. A click that closes a menu now only closes it, and the same fix covers the canvas, rulers and timeline.
- Picture fill: choosing Picture when the course has no pictures opens the file picker, and the Image menu always offers "Choose a file…". The chosen picture fills the shape or button; it is no longer placed as a separate picture. The menu lists only pictures, not audio or video, and cancelling the file picker leaves the fill as it was.
- Gradient colours are labelled Colour 1, Colour 2 and so on, with "Add colour", and each row fits inside the panel (its position and remove button used to run off the edge).
- Radio group options fit inside the Format panel. One option row was wider than the panel and pushed every field below it past the edge.
- The timeline ruler labels every 1, 2, 5 or 10 seconds as space allows, so labels on a long slide no longer overlap.
- The colour field for an object state is wide enough to show the whole colour code.

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
