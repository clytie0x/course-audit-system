# UI redesign and load-time optimisation

Date: 2026-11-22. Status: approved mockup, design pending approval.
Visual reference: [2026-11-22-ui-redesign-mockup.html](2026-11-22-ui-redesign-mockup.html) (open in a browser; the bottom-right panel switches fonts and the before/after-upload view). The mockup's course data is sample data.

## Why

The owner finds the current UI unprofessional and poor across screen sizes. Measured problems:

- Eight result sections stack in one long column; a 360px sidebar keeps a large upload box after analysis, and its lower half is empty on desktop. On a phone the results sit below the upload box and the how-it-works card.
- The headline "138 credits" gives no target and hides that the student cannot graduate yet (free-elective overflow does not count toward other categories).
- Category cards and the "Course details" accordion show the same numbers twice.
- Thai labels use `uppercase tracking-widest`, which spreads Thai glyphs apart (`hero.rs:17`, `sidebar.rs:301`, `sidebar.rs:315`, `missing_requirements.rs:94`).
- Load: the wasm is 562 KB gzip (569 KB as served by Pages); `pdf.min.js` (90 KB gzip) is a render-blocking script in `<head>` even though it is needed only after a file is chosen; fonts start loading only after the wasm renders (~690 ms in a warm-cache trace).

## Goals

- One layout that works from 360px phones to wide desktops, matching the approved mockup.
- The first screen after analysis answers: can I graduate, how many credits count, what is left.
- Thai typography done right; a better Thai/Latin font pairing.
- Smaller, faster first load without changing audit results.

## Non-goals

- No change to audit rules, parsing rules, curricula data or the SIS scraper.
- No new features beyond what the mockup shows (no accounts, no drag-and-drop planner, no prerequisite graph).
- No change to privacy behaviour: everything stays client-side; localStorage only when the user opts in.

## Anti-goals

- Any difference in parsed courses or audit numbers for the same PDF.
- A layout that needs a second navigation system on mobile.
- Horizontal page scroll at any width from 360px up.

## Approaches considered

1. **Tabs (chosen).** Sticky header, status banner, KPI row, then tabs: Overview, Courses, Semesters, Plan, Offerings. Same structure on every screen; the tab bar scrolls horizontally on phones. Fits the existing component split with the least new structure.
2. Dashboard with a side navigation. Most app-like, but phones need a separate bottom bar, so two navigation systems to maintain.
3. Keep the current layout and fix the five issues. Smallest diff, but the long single column, the main complaint, stays.

The owner reviewed mockups of all three, asked for research-backed polish, and approved the tabbed mockup.

## Design

### Fonts

- UI text: **Anuphan** (Cadson Demak, OFL), loopless Thai whose Latin is derived from IBM Plex Sans, so Thai and Latin read as one family. Weights 300–700, variable.
- Course codes: **IBM Plex Mono** 400/500 (OFL).
- Self-host the woff2 files in `vendor/fonts` (Thai + Latin subsets) with their licence files, replacing Geist, Geist Mono and Noto Sans Thai, which become unused. Add `<link rel="preload" as="font" crossorigin>` for the Anuphan files in `index.html`.
- Body line-height ≥ 1.6, headings ~1.35. No `uppercase` or letter-spacing on Thai text. Numbers use `tabular-nums`.

### Colour and surfaces

Zinc neutrals as today, one blue accent for in-progress bars and the active state, green for complete, amber for "still missing". Values are in the mockup's `:root` and `[data-theme="dark"]` blocks; map them into `tailwind.config.js` tokens. Progress fill against track must reach 3:1 contrast (WCAG 1.4.11) in both themes.

### Layout (see mockup for exact spacing)

**Before upload (empty state, full width, no sidebar):** headline, one-line explanation, large drop zone, curriculum select (default: detect from student ID), Analyse button, privacy line, three how-it-works steps, and the "save results in this browser" checkbox. Loading and error states render in this same area.

**After analysis:**

1. Sticky header: logo, programme pill, file chip with "Change file" (returns to the empty state), EN/TH toggle, theme toggle. On phones the file name hides and only the chip button remains.
2. Page head: title, programme · curriculum · cohort line, actions Export JSON and Print.
3. Status banner: one sentence stating whether the student meets the curriculum; when not, how many credits are missing and in which category, plus the overflow note when total credits exceed the requirement but categories do not. Registration-mode notice keeps its current amber banner.
4. KPI row (container: 4 columns at ≥1024px, 3 at ≥700px with the first card full width, 2 below that with the first and last full width):
   - **Counted credits** `Σ min(collected, required)` over categories, out of `Σ required`, with a progress bar and footnote "total earned N · overflow in free electives M".
   - **GPA** with the curriculum minimum (`min_gpa`). No honours tag: the code has no honours rules and a GPA-only tag would be wrong for students with F or retakes (builder's call, see below).
   - **Still to take**: missing credits and the categories they come from.
   - **Co-op prep hours**: `x / 30 h` from the existing co-op hours card (localStorage, opt-in); "not entered yet" with a link to that card otherwise. This card is the same for every curriculum; it does not guess which capstone option the student takes.
5. Tabs (`role="tablist"`, arrow-key navigation):
   - **Overview**: left, "Progress by category" rows (name, status tag, `collected / required`, bar); the major row expands into the groups its curriculum actually has (2564: basic science, core, capstone, major electives; 2569 has no capstone group because project and co-op sit in its core, so only three groups show; a group with zero required credits is omitted). Capstone follows the existing auditor rule: one option is required, the target follows the option taken, otherwise the first option. The mockup's "3 / 9" and "required" tag on co-op are sample data and wrong for 2564; free electives that exceed the requirement show the counted part solid and the overflow hatched with a "+N over" tag; legend below. Right, "What to do next" (missing courses and credit gaps, each with credits and, when offered, the next term it is offered, linking to the Offerings tab), "GPA by semester" bar chart, and the existing co-op prep-hours card.
   - **Courses**: one collapsible section per category with a course table (code, name, term, credits, grade). This replaces both the category cards and the old accordion.
   - **Semesters**: responsive grid of semester cards (1/2/3 columns) with term credits and GPA; in-progress terms tagged.
   - **Plan**: the What-If form in one row on desktop, stacked on phones; planned-course table; projection strip (counted credits, projected GPA, status). Prerequisite warnings stay. Planned courses affect only this tab: the banner, KPIs, Overview, Courses, Semesters, Export JSON and the saved `audit_v1` all use the real transcript. This needs two audit results (real, and real + planned); today `app.rs` writes the merged one into `audit_result`.
   - **Offerings**: term select, search, "only courses I still need" filter, table.

   The Offerings panel also stays available before upload, below the how-it-works steps in the empty state (without the "only courses I still need" filter), as it is today.
6. Footer line with the save-results toggle.
7. Print stylesheet shows all panels in sequence and hides chrome.

### Data needed from `audit-core`

- `Category` gains an optional list of groups `{ name, required_credits, collected_credits }`, filled for the major from the curriculum's own groups and empty elsewhere; `#[serde(default)]` keeps old saved JSON loading. Counted credits, overflow and missing-by-category are derived in the UI from existing fields.
- `MissingCourse` gains optional structured fields `code: Option<String>` and `credits: Option<f32>` (`#[serde(default)]`), filled where the auditor already knows them, so "What to do next" does not parse the English `description`. Rule descriptions (cluster conditions) may stay English in Thai mode, as today.
- None of this changes which courses count where or any credit total.
- Next-offered term for a missing course comes from the existing offerings index in the UI, not from `audit-core`.

### Load-time changes

- Replace `regex` with `regex-lite` in `audit-core`. Measured in a scratch build: wasm 1,497,752 → 800,883 bytes raw, 562,441 → 307,598 gzip; `cargo test` 64/64 pass; parse output for the owner's TH and EN transcripts byte-identical to the current parser (55 courses, cohort 2567). `regex-lite` treats `\s` and `\b` as ASCII, so add unit tests covering a no-break space between fields and a course code adjacent to Thai text, and normalise U+00A0 to a space before matching if the first test fails.
- Load `pdf.js` on demand inside `extractTextFromPDF` (inject the script once with its existing SRI hash, set `workerSrc`, then extract); remove the blocking `<head>` script.
- Preload the two primary font files.

## Constraints

- Rust + Leptos 0.8 CSR + Tailwind via Trunk; keep the existing component boundaries where possible and the `AppState` context.
- Must pass `cargo fmt --check`, both `cargo clippy` runs with `-D warnings`, `cargo test`, `trunk build --release`, and CI on the PR.
- Bilingual: every new string gets Thai and English, following the existing `is_thai` pattern.
- Never commit transcripts or personal data; the mockup and tests use sample data only.

## Verification

- Unit tests for the new `Category` groups and the regex edge cases.
- Browser check at 375, 820 and 1280px, light and dark: no horizontal scroll, tabs keyboard-navigable, focus rings visible.
- The owner's own TH transcript before and after: same courses, categories, credits and GPA.
- Record wasm size (gzip) before and after in the PR description.

## Calls made without the owner (check these)

- Planned courses stay inside the Plan tab and are not exported or saved as real results.
- No honours tag.
- Fourth KPI is co-op prep hours, not a capstone-option guess.
- Offerings stay searchable before upload.
- `MissingCourse` gets structured `code` and `credits`.
- A curriculum chosen by hand wins over auto-detection; "Change file" clears the result and the plan; failed and withdrawn attempts show in the Semesters tab; the EN/TH toggle stays reachable on phones.

## Left to the builder

- Exact Tailwind class choices and component file split for the new tabs.
- Icon set (inline SVG as today).
- Wording of the status banner for edge cases (registration mode, no GPA, curriculum minimum GPA not met).
