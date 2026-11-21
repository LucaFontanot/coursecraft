# Aurelie feedback, round 2 (tested on 0.2.4, Windows 10)

Source: `docs/user-feedback/Aurelie Feedback V02.pdf`. Thank Aurelie in the changelog of the release that ships these fixes.

Several of these were reported in round 1 and marked fixed, yet still fail for a real user. So the rule for every item: **first reproduce it through the real user path** (the UI test harness: click the actual button, type into the actual field, drag on the actual canvas; or the real player/runtime path), see it fail, find the root cause, fix it, and keep that test. A test that only calls an inner helper does not count as a reproduction.

| # | Area | What the user saw | Expected |
|---|---|---|---|
| A1 | Text format panel | Font size cannot be changed: typing a value resets it, dragging does nothing. Same for the spacing field just below it. | Every numeric field in the text section (size, letter spacing, line spacing, paragraph spacing) accepts typed and dragged values, for one object and for several selected objects, with inline editing open or closed. |
| A2 | Gradients | Linear and radial gradients only use the first two colours; a third colour is ignored. Colours cannot be reordered. | Any number of colour stops render (editor canvas, preview, published web player, thumbnails) for shapes, buttons and every other object with a fill. Stops can be reordered (move up/down) and their positions edited. |
| A3 | Animations | Slide with two objects: one has a Fly out exit, the other a Disappear exit at the end of the slide. After adding a slide transition, the Fly out exit no longer plays (the other still does). | Exit animations play regardless of the slide transition, for every exit effect type. |
| A4 | Timeline | Moving the blue playhead in the editor changes the time counter but the canvas does not show the slide at that time. | Scrubbing shows each object's state at that time (visible or not by its start/end, entrance/exit animation progress), and play in the timeline plays it. |
| A5 | Drawing shapes | While dragging to draw a shape, the outline is always a rectangle (also for heart, circle, star...). | The drag preview shows the outline of the chosen shape. |
| A6 | Insert ▸ Interaction | Works the first time; the second time it opens as a tiny window showing only the title bar. | Opens full size every time. Check every other dialog that closes itself after a choice. |
| A7 | Video | Dropped a few-second video from the PC onto a slide; it imported, but clicking it froze and crashed the app (Windows). | Selecting a video never freezes or crashes; on a decode problem show a placeholder and a plain message. |
| A8 | Windows install | Could not find where CourseCraft is installed; had to run the downloaded file again. (The Windows download is a bare portable .exe.) | A real Windows installer (Start menu entry, uninstall), plus a clear note that the .exe is portable. |
| A9 | Slide layers | Editing a layer tab shows the base slide's content mixed with the layer, hard to read. | A toggle to show the base slide normally, dimmed, or hidden while editing a layer. |
| A10 | Question editing | The question editor is only in the bottom panel; she expected it in the right side panel. | The same question editor is also reachable from the right panel when a question slide or question object is selected. |
| A11 | Paint (format painter) | Still does nothing, even with an object selected. | Select an object, click Paint, click another object: its format is copied. A status hint says what to do; Esc cancels. |
| A12 | Preview window | Cannot close the preview by clicking outside it; only the X works. | Clicking outside the preview (on the dimmed backdrop) or pressing Esc closes it. |
| A13 | PowerPoint import | A deck exported from Canva: some slides blank, text kept but all formatting and colours lost, pictures missing, shapes appear as stray lines. Another deck was better but not right. | Common PowerPoint constructs import faithfully: grouped shapes (group transforms), pictures (in `p:pic` and as shape picture fill), theme/scheme colours, text run size/colour/bold/font, custom geometry, slide backgrounds, content inherited from layouts/masters. |
