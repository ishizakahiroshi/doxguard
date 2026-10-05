# Independent review: article #20261005-007

Review date: 2026-10-05 UTC. Reviewer did not implement the article or figures.

## Target and decision

- Repository: ishizakahiroshi/doxguard
- Fixed baseline: bda21dd74172c5cc6e05d438b9a6b26f6079c4a5
- Initially reviewed candidate: a159f0a370670817dab100203aef001f2ceda77a
- Scope: the complete baseline-to-candidate change, verified through GitHub compare. 44 files, all under docs/bot/article/si-domain/; 34 text files and 10 PNG files.
- Initial result: no critical, high, or medium factual/content defect found. Two low-severity improvements were sent to the producer and await delta review below.
- This is not an unconditional end-to-end acceptance. Browser-rendered HTML overflow, the repository doxguard scan, final PR checks, and the local cat-overlay acceptance are not passed by this review.

## Findings

### Low: reader-facing infographic points to an unlinked production file

The footer of 02_infographic.png says that links and calculations are in the body and SOURCES.md. A note reader has no direct link to that production file. Replace the instruction with 「数値・リンク・算式は本文へ。」 while retaining the adjacent source names and the causation caveat. This does not change the numeric claims.

### Low: update the WIPO citation to the current overview

The cited WIPO Overview 3.0 page explicitly states that it has been superseded by 3.1. The current overview retains the relevant principle in section 3.1.1: resale for profit alone does not establish bad-faith targeting of a trademark. Update the draft, source ledger, and source generator to the current official URL and version. No legal conclusion about this particular holder should be added.

- Original: https://www.wipo.int/en/web/amc/domain-name-disputes/overview/3.0
- Current, independently read: https://www.wipo.int/en/web/amc/domain-name-disputes/overview/index

## Full scope read

The exact README.md, FACTS.md, BRIEF.md, REVIEW.md, PROGRESS.md and publication.json were read. Baseline copies were retrieved; README/FACTS/BRIEF/REVIEW only gain a final blank line. All newly added text files were read in full, including draft.md, x-post.md, SOURCES.md, all four HTML/SVG/rendered-text sets, capability-test.html, build_sources.py, render_figures.py, validate.py, image-prompts.md, reconciliation.md, registry-stance-evidence.md, si-primary-research.md, all three primary-data/event JSON files, render-checks.json and validation.json. The baseline PROGRESS and publication status changes were inspected.

The GitHub tree for a159f0a was compared with local files using Git blob hashing, SHA1("blob " + byte length + NUL + content). All 44 local files matched their committed blobs. No unrelated product source or other article changed. The review itself did not edit article deliverables or Git state.

## Claim, source, and editorial checks

- White House EO14434 and Federal Register document 2026-20321 were independently read. The signature date is 2026-09-29, publication is 2026-10-02, and the scope is executive-branch official communications and non-statutory documents, subject to law. Historical documents need not be rewritten. The existing AI definition initially applies; the science/technology adviser has 60 days to propose legislative language. The article does not turn the order into a worldwide/private-sector mandate or evidence that technical superintelligence exists.
- Register.si's growth page and its official 200,000-domain statement were independently read. The statement supports pre-existing growth, extra interest from the naming debate, wholesale/retail distinctions, and caution about national profits. The archived chart8157 extract records August 3,515 and September 46,066. The article and figures consistently use monthly new registrations and 46,066 / 3,515 = 13.10554765, rounded to 13.1. September explicitly includes dates before the order. These are not active-site, profit, or resale counts. Fresh retrieval of the embedded raw chart was not completed by this reviewer; see limits.
- IANA's .si/.ai/.com records independently confirm their distinct TLDs and managers. spacexsi.com is outside .si registration totals.
- The live sales landing page independently displayed $174,888 on review day. The article labels it a asking price and supplies the observation date; it does not invent a buyer, transaction, holder identity, or purpose. The committed RDAP excerpt states registration on 2025-07-25. The distinction between original registration and the current holder's acquisition date is preserved. Fresh RDAP retrieval was not completed by this reviewer; see limits.
- Reuters via CNA independently confirms a stated intention to rename SpaceXAI to SpaceXSI and no completed account change/timeline at publication. Direct X page retrieval failed for this reviewer. The author's 「たけえ」 reaction is also supplied in the fixed FACTS instructions; no additional personal scene or activity is invented.
- The government 2026 budget PDF was independently read at PDF pages 67 and 70, with corresponding archived table pixels inspected. The 2024 Actual column uses 104,253,510 / 469,861,457 EC dollars; 2025 End of Year Projection uses 219,220,239 / 560,360,763; 2026 Proposed Estimate uses 253,557,731 / 595,881,631. Independently recalculated shares are 22.2%, 39.1%, 42.6%. Neither forecasts nor recurrent-revenue denominators are relabeled as actuals or expenditure-budget totals.
- The newer EC$230,499,740.50 cash-receipt report was independently read at Anguilla Focus. It remains explicitly secondary, and no unsupported actual-revenue percentage is computed from it. The older US$93 million/47% series is not used.
- IMF 2024 Annex IV Box 1 independently supports 2022 about 5%, 2023 EC$87 million and more than 20%, and subsequent renewals. IMF 2026's official search-retrieved text supports around 40% recurrent-revenue exposure and recommended stabilization/governance/investment/diversification measures; direct opening failed. The article does not claim a fund is already established.
- The 2024 government budget speech independently supports dependency caution and infrastructure/airport investment. Identity Digital's January 15 and March 27, 2025 releases support migration and auctions/DropZone; no unsupported revenue-share percentage is added.
- Registry eligibility/trademark explanations, ARDS rules, ICANN UDRP, and WIPO were independently read. The article does not apply .si's system to .com or determine liability from a price tag. The two systems' different scopes are preserved.
- Both related note articles were independently read to check the narrow summaries used here. Their unseen content is not invented.
- The source ledger covers all 25 unique body URLs. Primary sources and reporting are labeled, dates and currencies are consistent, and adjacent URL lines cover numerical claims. Opinion and possible future behavior are distinguished from observed facts. All four required personal opinions are marked with ※ and 「個人的には」. No .ai decline, completed rename, fabricated transfer, or fabricated ownership is asserted.

## Pixel inspection and format

All four final PNGs were opened with view_image and inspected at their actual 1600 × 900 dimensions. Japanese text and numbers are readable, with no apparent glyph corruption, clipping, missing artwork, recognizable real-person faces, or corporate logos.

- 01_hero.png: warm teal/sand/coral palette, .ai sign and .si wave, no cat. The required bottom-right rectangle (1260,650)-(1600,900) is unobstructed and mechanically uniform in color. Cat overlay remains a local step.
- 02_infographic.png: three distinct columns; government scope, 3,515 to 46,066, and 42.6% agree with the body. The forecast/denominator and no-confirmed-decline qualifiers are visible. The source-file pointer is the low finding above.
- 03_illustration.png: anonymous illustrated observer; US$174,888 is explicitly a desired price, not a completed sale. The .com distinction, observation date, prior registration date, and uncertainty about the current holder are visible.
- 04_fig.png: common zero-based 0-50,000 axis, equal bar heights, correctly proportional bar lengths, counts and month labels match the data, and the pre-order-period caveat is present. Footer is legible in the PNG despite its tight text box.

The draft has one H1, conversational H2 headings, short polite paragraphs, all four five-line image markers and matching files, the infographic before the first H2, and explanatory text after each body figure. The exact required footer matches the brief. Related reading is before the final closing section and footer. The last line has 13 hashtags. No prohibited dash appears in the article or rendered figure text. Body count excluding URLs, markers, footer and whitespace: 4,585 characters. X draft: account frontmatter is correct, no hashtag, final publication-URL placeholder, weighted count 195 including URL weight 23.

## Verification executed

1. GitHub compare_commits with the exact fixed baseline and a159f0a; recursive tree read and all-file Git blob hash comparison.
2. Read all changed text and exact baseline instruction files; comparison confirmed final-newline-only instruction edits.
3. Copy the candidate directory into an isolated temporary verification directory, then execute runpy.run_path('src/validate.py', run_name='__main__') there. Result: scoped assertions pass. This is explicitly not repository doxguard.
4. Execute runpy.run_path('src/render_figures.py', run_name='__main__') in that copy. Result: all text boxes fit at scale 1, and regenerated SHA-256 hashes for all four PNGs exactly match the candidate.
5. Independently compare the requested footer string, recalculate the three financial ratios and monthly multiplier, count final tags, and inspect image dimensions/reserved-color region.
6. Independently read critical government, registry, IANA, IMF, WIPO/ICANN, operator and cited reporting pages as detailed above; inspect final PNG pixels plus the two financial-table evidence crops.

## Residual limitations and next acceptance steps

- Browser layout/overflow of the final HTML was not run. The producer reports a rejected local file URL; this review did not bypass it. PyMuPDF fitting and PNG pixel inspection are separate checks and do not count as browser overflow PASS.
- The repository doxguard executable/Rust toolchain is not available in this artifact workspace. A limited structural secret/private-path scan passed; a private watchlist was neither obtained nor used. Repository CI must be checked for the final submitted head.
- Fresh raw chart/RDAP read: a read-only urllib request group for the official chart page, Verisign RDAP, and sales page initially yielded no output, then write_stdin failed with “Unified exec process failed: automatic approval review was cancelled.” No reason classified it as missing authorization. No retry or alternate raw retrieval was attempted after that cancellation. The existing commit-bound chart/RDAP extracts were reviewed for consistency, but this is not an independently refreshed source-data capture. The sales price was separately visible in the already authorized web read.
- Direct X pages were unavailable to this reviewer; rename context was checked against Reuters/CNA, and the author's reaction against the fixed user-supplied FACTS. The producer's stated browser/oEmbed inspection is not counted as this reviewer's independent X inspection.
- IMF 2026 was verified from official search-indexed text; direct open failed. The source ledger already discloses this.
- The local cat overlay, local final acceptance, final Draft PR state and exact-head CI are later handoff checks. Publication remains unauthorized and outside this review.

## Delta review

Pending producer's follow-up SHA for the two low-severity changes. No unconditional PASS for unrun checks is implied.
