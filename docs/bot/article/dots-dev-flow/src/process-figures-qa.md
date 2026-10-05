# Process figures: QA and reproduction

Article: #20261005-002
Date: 2026-10-05

## Files and method

- Final diagram 1: `../03-dots-dev-flow-fig1.png`
- Editable diagram 1: `03-dots-dev-flow-fig1.svg`
- Final diagram 2: `../05-dots-dev-flow-fig2.png`
- Editable diagram 2: `05-dots-dev-flow-fig2.svg`
- Rebuild: `python src/build_process_figures.py` from the article directory.
- The deterministic script uses one vector scene definition for SVG text/shapes and Pillow PNG rendering. It supersamples at 2× then downsamples to 1600×900 using Lanczos.
- Installed Noto Sans CJK JP regular/bold is used. No dependency installation, network request, generated raster illustration, or mascot material is used for these diagrams.
- The source and PNG hashes, byte sizes, dimensions, alt text, font details, palette, and text bounds are recorded in `process-figures-receipt.json`. Running the script twice produced identical PNG hashes.

## Visual inspection

Both full 1600×900 final PNGs were separately opened with the builtin image viewer and inspected.

- Japanese glyphs are present; no tofu, missing glyphs, clipping, or text/arrow collisions were observed.
- The title and main steps use large type and generous separation. Main diagram text is 26–42 px; titles 48–50 px. The small 23–25 px text is restricted to secondary labels. Phone landscape or zoom preserves legibility; fitting all of a 1600 px-wide detailed diagram into a narrow portrait column naturally reduces type size.
- Figure 1 has a clear left-to-right stage chain, a coral stop branch from the whole dots scope to a person, and a final independent review followed by human acceptance/integration. People appear at initial approval, final acceptance, and the stop branch only.
- Figure 1 is explicitly a reusable next-time model. Its footer states that the actual first independent review happened after initial integration, so the model is not presented as the historical sequence.
- Figure 2 distinguishes the severity counts (serious 1, medium 12, light approximately 20) from the test-name/assert-discrepancy axis (approximately 20), explicitly disallowing addition.
- Figure 2's reread result is serious 0 plus newly found medium 2, carried to the beginning of the next stage. It does not assert that all problems were removed.
- Figure 2 includes all three test weakness types from FACTS.md and the test-count progression 346 → 580 → 718.
- Both diagrams use the shared ivory/plum/coral/sage palette and contain no cats.

## Technical verification

- Both SVGs parse as valid XML and retain editable text and vector shapes.
- Both PNGs are RGB, exactly 1600×900, and below 3,000,000 bytes.
- All recorded text bounds remain inside the canvas.
- No file outside this article directory was modified.
