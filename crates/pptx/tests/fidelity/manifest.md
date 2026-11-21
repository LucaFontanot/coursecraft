# Fidelity deck manifest

Slide number -> contents. Deck: fidelity.pptx (16:9, 19 slides).

- **Slide 1**: Basic auto shapes: rect, rounded rect, ellipse, triangle, diamond, pentagon, hexagon, star 5, star 8, heart; row 1 no text, row 2 with text; solid slide bg; adjusted rounded rect / star
- **Slide 2**: Arrows (right/left/up/down/chevron/block arc), callouts (rect, rounded, oval), cloud, lightning, moon, smiley, cross, donut, flowchart shapes; mixed with/without text
- **Slide 3**: Right arrows rotated 0/30/60/90/180/270; flipH/flipV/both arrows with text; rotated rect with text 45; flipped callout with text; rotated ellipse, rotated triangle
- **Slide 4**: custGeom freeforms: polygon triangle, 5-point star polygon, bezier blob, heart via cubicBezTo, arcTo pie, multi-subpath (ring with hole), path w/h scaled vs shape extents, open path stroke only, python-pptx freeform
- **Slide 5**: Straight lines (horizontal, vertical, diagonal both directions), dashed variants, thick, arrow heads (triangle, stealth, oval, diamond, arrow), elbow and curved connectors joining shapes (flipped)
- **Slide 6**: Fills: solid; linear gradient 2 stops 0deg/45deg, 3 stops 90deg/135deg, 5 stops 0deg/270deg; radial gradient; transparency over stripes; no fill outline only; picture fill (stretch) in rect and ellipse; tiled picture fill; pattern fills (ltUpDiag, wdUpDiag, dotGrid, smCheck, horz, cross)
- **Slide 7**: Outline widths 0.5-16pt; colours; dash styles (dash, dot, dashdot, longdash, sysdot); arrowhead on shape outline n/a; outer shadows (soft, hard, large offset, on ellipse, on text box); thick outline on rounded rect
- **Slide 8**: Text boxes only: one paragraph mixing bold/italic/underline/strike/colours/sizes 10-60/superscript/subscript/fonts Arial, Georgia, Courier New, Calibri; theme fonts; text with no explicit font
- **Slide 9**: Alignment left/center/right/justify in bordered boxes; line spacing 80%/100%/150%/200% and exact pts; space before/after; anchors top/middle/bottom; normAutofit fontScale; spAutoFit; wrap none; rotated text box 20deg; vertical text vert270; text insets
- **Slide 10**: Bullet levels (default bullet, en dash, >> ), custom bullet characters (star, check, arrow) with colour and size, numbering 1. / a) / i. nested, hanging indents
- **Slide 11**: Theme accent1..6 via schemeClr (theme edited: accent1 E91E63 pink, accent2 00BCD4 cyan, accent3 FFC107 amber, accent4 673AB7, accent5 4CAF50, accent6 FF5722) next to explicit RGB twin; lumMod/lumOff tints; dk2/lt2; text and outline in schemeClr; theme fonts Georgia major / Trebuchet MS minor
- **Slide 12**: Pictures: plain png, jpg, cropped (left/right/top/bottom), rotated 15deg, flipped, PNG with alpha over stripes, picture behind text, picture with outline; 'plasma' image has TL/TR/BL/BR corner letters and yellow border so cropping is visible
- **Slide 13**: Group shapes: A simple group; B scaled group (child extents != group extents, 2x wide / 0.5x tall) with line children; C nested group inside group (both scaled); D group rotated 20deg + flipH. Each has a dotted outline rectangle showing where the group SHOULD land
- **Slide 14**: Tables: 5x4 with header row (default medium style 2 accent 1 -> pink theme accent1), banded rows, right aligned numbers, explicit cell fills, merged cell, bold text, different column widths, second table custom fills only
- **Slide 15**: Embedded video intro.mp4 via add_movie with generated poster image; plus the same poster as a plain picture beside it for comparison
- **Slide 16**: Slide background: linear gradient (dark blue to teal 3 stops, 60deg), with some shapes and text on top
- **Slide 17**: Slide background: stretched picture fill (plasma.png with corner letters), shapes and text on top
- **Slide 18**: Layout placeholders (title + body): positions and formats inherited from layout (title Georgia 44 bold red at 1.0,0.5; body 28pt blue with triangle bullets, level 2 italic green 20pt); plus a normal text box and a shape on the slide
- **Slide 19**: Hidden slide (show="0"): should import as hidden or at least not break numbering; contains text and a shape
