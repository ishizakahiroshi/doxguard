# Independent production image review

Article: #20261005-002  
Review date: 2026-10-05  
Scope: all five final PNGs, the supplied requirements and facts, and the retained editable sources. This is a same-product independent production review. It does not replace the requested final review and acceptance by the owner's separate-product/local reviewer.

## Result

Ready for final hand inspection. No unresolved production-image blocker remains after the infographic revision. No factual, chronology, Japanese-glyph, overlap, image-size, or mascot-placement defect was found in the five currently inspected PNGs. Detailed text still requires enlargement on narrow portrait screens; this is a documented zoom expectation, not an unconditional mobile-legibility pass. Published mobile rendering/zoom and final separate-product/local human acceptance remain unverified.

### M-01: portrait-width details require enlargement (mitigated; documented limitation)

- Initial finding: the first infographic used 24–31 px type in four narrow columns, which became about 5.9–7.6 CSS px at 390 px portrait width. Its initial SHA-256 was 9be7746626d93ff965a560d0582034a07c2fdfeadae93594d3914fd332ca99da; that version was inspected but not accepted as mobile-legible.
- Revision reinspected: opened the revised final infographic PNG, not only its SVG. Its labels now use 48 px type in a spacious two-column/two-row layout, with 32 px explanatory lines. At 390 px width the labels are about 11.7 CSS px; explanatory lines remain about 7.8 CSS px and the 27 px final note about 6.6 CSS px. No overlap, clipping, glyph defect, factual change, or sequence ambiguity was found. The retained generated art strip remains visually coherent.
- Remaining limitation: figure 1 uses 23–32 px for many essential details, including its chronology distinction. Figure 2 uses 23–31 px for explanatory information. At a 390 px-wide rendering of a 1600 px image, these become approximately 5.6–7.8 CSS px. Clean full-resolution spacing does not make every word comfortably readable in a portrait fit-to-column view.
- Mitigation verified in files: all three article drafts now contain a neutral instruction to enlarge the figures. The process/chronology, initial and reread counts, and three test-weakness types are also explained in readable prose. Zenn and Qiita retain descriptive image alt text; note retains descriptive insertion instructions. Therefore this is not a remaining production-image submission blocker.
- Final hand-inspection requirement: verify the real platforms' mobile rendering and image enlargement behavior, correct figure insertion and alt/caption treatment, and readable adjacent prose. No actual published page or mobile device was tested here. This production review is not final acceptance.

## Inspection performed

Read FACTS.md, BRIEF.md, REVIEW.md, and README.md in full before review. Reopened the revised infographic and checked the relevant enlargement/prose passages in all three drafts on 2026-10-05. Opened each final PNG separately through the actual image viewer and inspected its pixels, rather than relying on source SVGs or another worker's QA claim. Also opened the actual fixed mascot PNG and compared its appearance with the hero.

- Hero: Japanese title and supporting text are intact; strong contrast and broad margins; fixed cats appear at the lower right and do not compete with the title. The full title remains the dominant element. The scene contains no additional cats.
- Infographic: four steps and their sequence agree with the supplied facts. The environment stop is followed by moving verification to CI. Initial review counts are serious 1 and medium 12. The later serious-0 result is explicitly qualified by the new medium-2 note. No clipping or missing glyphs was seen.
- Figure 1: initial human approval, stages 0–4, CI, stop-to-human branch, separate-product full-line review, and human acceptance/integration are legible and connected at full size. The diagram explicitly describes the reusable next-time pattern; its footer correctly says the historical first independent review happened after integration. The same-product implementation reviewer and final separate-product reviewer are not conflated.
- Wordless illustration: distinct maker and reviewer, a connected sequence of work surfaces and a final full-sheet inspection. No text, numbers, logos, or cats were observed.
- Figure 2: serious 1 / medium 12 / light about 20, separate test-name/assert mismatch axis about 20, reread serious 0 / new medium 2, and tests 346 → 580 → 718 all agree with FACTS.md. The three test-weakness types are present. The separate mismatch count is not falsely added to the severity counts. The image does not assert zero remaining problems.
- Palette: all five consistently use ivory, plum, muted plum, coral, and sage. The preceding article's actual images were not provided or independently opened, so distinctness from that article was not verified here.
- No private repository identifiers, real hosts, Slack identifiers, PR numbers, commit hashes, or home paths appear in the rendered images.

## Final PNG measurements and inspected SHA-256

All final PNGs are exactly 1600×900 and below 3,000,000 bytes.

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| 01-dots-dev-flow-hero.png | 2,074,301 | 1d695ae6b9d817b66ad2622e6a61493cc7bde1c4d15ad96f62659dcbe611a994 |
| 02-dots-dev-flow-infographic.png | 796,849 | d58a858c821c0d1ff7005433a680a0b20c466dd58caa6fd12b4ce3725b59f26e |
| 03-dots-dev-flow-fig1.png | 330,673 | ab1743bccd92b806c8441f447a23a037de169ee4d5f9a7671b3ede830d2c4ca9 |
| 04-dots-dev-flow-illustration.png | 1,951,279 | d5f5921b8f742a15fbe7fcefe544036c210bc9dfac8c04c96a232c69f5c5314d |
| 05-dots-dev-flow-fig2.png | 374,421 | d016f119d7338d3c05d864b97aa0338c0d9404e7f5592ea4fc1f7a18845173bb |

## Source and mascot audit

- All four retained SVGs parse as XML and retain editable Japanese text: hero 7 text nodes; revised infographic 15; figure 1 34; figure 2 32.
- The hero compositor refers directly to ../assets/mascot_cats.png and only the hero contains that image element. The mascot's computed Git blob SHA-1 is 229e3a806e16cd636594704f91a193ac9b5c8fc8, matching the required supplied material. Its SHA-256 is 46ef71ab89f6203785a6cf9aababb69e9516f33d14c4bad76bd76576f5d0b91d.
- The compositor retains native generated raster inputs for hero, infographic artwork, and illustration. It overlays Japanese SVG text and the supplied mascot after generation; the illustration is resized to the final canvas. The images were not regenerated or edited during this review.
- Both figure SVG and final PNG SHA-256 values, and the renderer script SHA-256, match process-figures-receipt.json. The figure renderer is deterministic Python/Pillow/vector code and does not reference the mascot or generated artwork.
- Generation prompts and the three raster sources are present. The production lead reports that native image generation was used. This reviewer did not independently observe those generation calls; file presence and prompts alone are not proof of tool execution.

| Retained source | SHA-256 |
| --- | --- |
| hero.svg | ccc1ec07e71524ff927c905a093259c904d746a506ba4afb5c6ffb412d390aca |
| infographic.svg | 23006a7843ac1968dc191be9e9bbd8962659f630f5bebb2a5931a7ae1c065caa |
| 03-dots-dev-flow-fig1.svg | aafe0ace275d80859571ad7e5f8ca8a953bf682d9516996858c2c42b2235ef60 |
| 05-dots-dev-flow-fig2.svg | cab5f6d761c87dd376c27494c9e902fee288f0bafc20acbe155bed39c573a50b |
| compose_assets.py | 5f1144e1fbbed992894f3bca92d7c4e8c756fb318991dae37d84671ddfb65583 |
| hero-scene.png | 37ae3ea82e6b5d1a6bedf10628b75111e6fac169c911258a8d4b9c86daeb014b |
| infographic-art.png | 4686d3aea20912624394e97015bcdf513d0ad6b402cb0d92ca1bfc0d7d4e971f |
| illustration-source.png | e2f59e7037b715d4800b3beab3b25cf76eac4a0f0cace48ae46a761cf4abb277 |

No network access, dependency installation, pnpm execution, publication, or image modification was performed for this review. Source-to-PNG rebuild reproducibility was not independently rerun. Published-page rendering, actual image zoom behavior, complete article insertion/alt coverage, and final human acceptance remain outside this production-image review. The enlargement guidance and key prose equivalents were checked in the current drafts only.
