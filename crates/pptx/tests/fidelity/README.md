# PowerPoint Fidelity Reference Assets

- `fidelity.pptx`: 19-slide 16:9 test deck (18 visible slides + 1 hidden slide).
- `manifest.md`: Per-slide description of what each slide tests.
- `Slide1.png` … `Slide18.png`: Exact PowerPoint `File ▸ Export ▸ PNG` output (960×540) for the 18 visible slides. Slide 19 is hidden and skipped.

## Regenerating the reference PNGs

1. Open `fidelity.pptx` in Microsoft PowerPoint.
2. Choose **File ▸ Export ▸ PNG** (or **Save As…** → PNG, all slides).
3. Copy `Slide1.png` through `Slide18.png` into this directory (skipping `Slide19.png`).
