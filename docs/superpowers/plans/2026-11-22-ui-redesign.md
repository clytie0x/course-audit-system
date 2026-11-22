# UI Redesign and Load-Time Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the sidebar-plus-long-column UI with the approved tabbed layout, and cut first-load cost, without changing any audit result.

**Architecture:** `audit-core` gains small, serde-defaulted data (major groups, structured missing items) and pure summary helpers with unit tests. The Leptos app keeps `AppState` but splits the audit into a real result and a plan result, drops the sidebar, and renders an empty state or a results shell (banner, KPIs, five tabs) built from the existing components where they fit.

**Tech Stack:** Rust, Leptos 0.8 CSR, Tailwind 3 via Trunk, `regex-lite`, pdf.js 3.11.174 (vendored), self-hosted Anuphan and IBM Plex Mono.

**Spec:** `docs/superpowers/specs/2026-11-22-ui-redesign-design.md` (visual reference: `docs/superpowers/specs/2026-11-22-ui-redesign-mockup.html`; its course data is sample data).

## Global Constraints

- Every user-facing string has Thai and English via the existing `is_thai` signal pattern.
- No `uppercase` and no `tracking-*` class on any element that can render Thai.
- Body line-height ≥ 1.6, headings ~1.35; numbers use `tabular-nums`.
- Progress fill vs track contrast ≥ 3:1 in light and dark.
- No horizontal page scroll at any width ≥ 360px.
- Never commit a transcript or personal data; tests use synthetic courses.
- Gates before every commit that touches Rust: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo clippy --target wasm32-unknown-unknown -- -D warnings`, `cargo test --workspace --all-targets`. Before the PR also `trunk build --release --public-url /course-audit-system/`.
- No change to which course counts where or to any credit total (anti-goal).
- Work on branch `ui-redesign`; one PR at the end; wait for CI; ask the owner before merging.

## Review Focus

1. A saved `audit_v1` from the current release (no `groups`, no `code`/`credits`) must still load and render after upgrade. Test in Task 2 and Task 3.
2. Curriculum 2569 has no capstone; its major must show three groups, not an empty "capstone 0/0" row. Test in Task 2.
3. Registration-mode PDFs (no grades, in-progress courses) give `gpa: None`; banner and GPA card must render "—", not panic or show 0.00. Test for the summary helper in Task 2; browser check in Task 10.
4. A no-break space or Thai text next to a course code in extracted text must parse as today after the `regex-lite` switch. Tests in Task 1.
5. Planned What-If courses must not leak into the banner, KPIs, Export JSON or `audit_v1`. Browser check in Task 6 and Task 10.

---

### Task 1: Switch the parser to `regex-lite`

**Files:**
- Modify: `crates/audit-core/Cargo.toml` (replace `regex = "1.10"` with `regex-lite = "0.1"`)
- Modify: `crates/audit-core/src/parser.rs:13` (`use regex::Regex;` → `use regex_lite::Regex;`) and its test module

**Interfaces:**
- Produces: no API change; `parse_transcript` and `detect_cohort` keep their signatures.

- [ ] **Step 1: Write the edge-case tests** in `parser.rs` tests:

```rust
#[test]
fn grade_line_with_no_break_space_parses() {
    let text = "344-111\u{a0}PROGRAMMING CONCEPTS\u{a0}6\u{a0}6\u{a0}A";
    let (courses, _) = parse_transcript(text);
    assert_eq!(courses.len(), 1);
    assert_eq!(courses[0].code, "344-111");
    assert_eq!(courses[0].grade, "A");
}

#[test]
fn course_code_after_thai_text_parses() {
    let text = "ภาคการศึกษา 1/2567 344-111 PROGRAMMING CONCEPTS 6 6 A";
    let (courses, _) = parse_transcript(text);
    assert_eq!(courses.len(), 1);
    assert_eq!(courses[0].semester.as_deref(), Some("1/2567"));
}
```

Check the exact `ParsedCourse` semester format against an existing parser test before fixing the expected value; keep whatever the current parser produces.

- [ ] **Step 2: Run them on the current `regex` crate** — `cargo test -p audit-core grade_line_with_no_break_space course_code_after_thai` — and record which pass. Both define today's behaviour; adjust an assertion only if the current crate disagrees, so the test pins current behaviour.
- [ ] **Step 3: Swap the dependency and import.**
- [ ] **Step 4: Run** `cargo test -p audit-core`. Expected: all pass. If the no-break-space test fails, add `let text = &text.replace('\u{a0}', " ");` at the top of `parse_transcript` and `detect_cohort` and rerun.
- [ ] **Step 5: Measure** `trunk build --release --public-url /course-audit-system/` then `gzip -9c dist/*_bg.wasm | wc -c`. Expected ≈ 307,598 (was 562,441). Note both numbers for the PR.
- [ ] **Step 6: Run all gates, commit** `Parser: regex-lite instead of regex (wasm 562 KB → ~308 KB gzip)`.

### Task 2: Major groups and summary helpers in `audit-core`

**Files:**
- Modify: `crates/audit-core/src/models.rs` (`Category`, new `CategoryGroup`, `impl AuditResult`, `impl Category`)
- Modify: `crates/audit-core/src/auditor.rs` (`MajorOutcome`, `audit_major`, `build_audit`)
- Test: `crates/audit-core/src/auditor.rs` and `models.rs` test modules

**Interfaces:**
- Produces:
  - `pub struct CategoryGroup { pub name: String, pub required_credits: f32, pub collected_credits: f32 }` (Serialize, Deserialize, Clone, Debug, PartialEq)
  - `Category.groups: Vec<CategoryGroup>` with `#[serde(default)]`
  - `impl Category { pub fn counted_credits(&self) -> f32 /* min(collected, required) */; pub fn overflow_credits(&self) -> f32 /* max(collected - required, 0) */; pub fn is_complete(&self) -> bool }`
  - `impl AuditResult { pub fn counted_credits(&self) -> f32; pub fn required_credits(&self) -> f32; pub fn earned_credits(&self) -> f32 /* = total_credits */ }`
  - Group names (English, used as keys by the UI for Thai labels): `"Basic Science"`, `"Core Courses"`, `"Capstone"`, `"Major Electives"`.

- [ ] **Step 1: Write failing tests** in `auditor.rs` tests:

```rust
#[test]
fn major_groups_sum_to_major_collected() {
    let r = build_audit(&[parsed("322-101", "A", 3.0), parsed("344-331", "A", 3.0)], cs64());
    let major = category(&r, "Major Courses");
    let names: Vec<_> = major.groups.iter().map(|g| g.name.as_str()).collect();
    assert_eq!(names, ["Basic Science", "Core Courses", "Capstone", "Major Electives"]);
    let sum: f32 = major.groups.iter().map(|g| g.collected_credits).sum();
    assert_eq!(sum, major.collected_credits);
    let req: f32 = major.groups.iter().map(|g| g.required_credits).sum();
    assert_eq!(req, major.required_credits);
}

#[test]
fn cs69_has_no_capstone_group() {
    let r = build_audit(&[parsed("322-101", "A", 3.0)], curriculum("cs2569"));
    let major = category(&r, "Major Courses");
    assert!(major.groups.iter().all(|g| g.name != "Capstone"));
    assert_eq!(major.groups.len(), 3);
}

#[test]
fn only_major_has_groups() {
    let r = build_audit(&[parsed("322-101", "A", 3.0)], cs64());
    assert!(category(&r, "General Education").groups.is_empty());
    assert!(category(&r, "Free Electives").groups.is_empty());
}
```

And in `models.rs` tests:

```rust
#[test]
fn counted_credits_caps_each_category() {
    // gen ed 30/30, major 87/96, free 21/6 -> counted 123 of 132, earned 138
    let r = AuditResult { total_credits: 138.0, categories: vec![cat(30.0, 30.0), cat(96.0, 87.0), cat(6.0, 21.0)], missing_subjects: vec![], gpa: Some(3.81), attempts: vec![] };
    assert_eq!(r.counted_credits(), 123.0);
    assert_eq!(r.required_credits(), 132.0);
    assert_eq!(r.categories[2].overflow_credits(), 15.0);
    assert!(!r.categories[1].is_complete());
}

#[test]
fn old_saved_result_without_groups_loads() {
    let json = r#"{"total_credits":3.0,"categories":[{"name":"Major Courses","required_credits":96.0,"collected_credits":3.0,"courses":[]}],"missing_subjects":[{"category":"Core Courses","description":"344-111 - X"}],"gpa":null,"attempts":[]}"#;
    let r: AuditResult = serde_json::from_str(json).unwrap();
    assert!(r.categories[0].groups.is_empty());
    assert_eq!(r.gpa, None);
}
```

(`cat(required, collected)` is a local test helper building a `Category` with empty courses and groups. Match the real `AuditResult` field list; add any field the struct has that the JSON above lacks only if it is already `#[serde(default)]`.)

- [ ] **Step 2: Run** `cargo test -p audit-core`. Expected: the new tests fail to compile (`groups`, `counted_credits` missing).
- [ ] **Step 3: Add `basic_credits`, `core_credits`, `capstone_credits`, `capstone_required` to `MajorOutcome`**; accumulate them where `completed_credits` is accumulated today (basic loop, core loop, capstone loop; `capstone_required` is the existing `capstone_credits` target variable). `completed_credits` stays their sum.
- [ ] **Step 4: Build `groups` in `build_audit`** for the major `Category`: Basic Science (`major.basic_science.required_credits`, `mj.basic_credits`), Core Courses (`major.core_courses.required_credits`, `mj.core_credits`), Capstone (`mj.capstone_required`, `mj.capstone_credits`; omit when `capstone_required == 0.0`), Major Electives (`mj.elective_required`, `elective_credits` — already capped). Other categories get `groups: Vec::new()`. Add the `models.rs` helpers.
- [ ] **Step 5: Run** `cargo test -p audit-core`. Expected: all pass, including every pre-existing test (credit totals unchanged).
- [ ] **Step 6: Gates, commit** `audit-core: major groups and counted-credit helpers`.

### Task 3: Structured fields on `MissingCourse`

**Files:**
- Modify: `crates/audit-core/src/models.rs:36` (`MissingCourse`)
- Modify: `crates/audit-core/src/auditor.rs` (the 19 `MissingCourse { .. }` literals)

**Interfaces:**
- Produces: `MissingCourse { category: String, description: String, #[serde(default)] code: Option<String>, #[serde(default)] credits: Option<f32> }`, deriving `Default`.

- [ ] **Step 1: Write failing tests** in `auditor.rs`:

```rust
#[test]
fn missing_core_course_has_code_and_credits() {
    let r = build_audit(&[], cs64());
    let m = r.missing_subjects.iter().find(|m| m.code.as_deref() == Some("344-111")).unwrap();
    assert_eq!(m.credits, Some(6.0));
    assert_eq!(m.category, "Core Courses");
}

#[test]
fn missing_elective_credit_gap_has_credits_no_code() {
    let r = build_audit(&[], cs64());
    let m = r.missing_subjects.iter().find(|m| m.category == "Major Electives").unwrap();
    assert!(m.code.is_none());
    assert!(m.credits.unwrap() > 0.0);
}
```

- [ ] **Step 2: Run, expect compile failure.**
- [ ] **Step 3: Add the fields and `Default`.** Fill `code` and `credits` where the literal has a single curriculum course in hand (basic, core, non-credit required with `credits: Some(0.0)`, GenEd slot misses that name one course); fill only `credits` for credit-gap misses (major electives, overall major, free electives, GenEd pools); leave both `None` for choice and rule misses (capstone "Choose 1", cluster rules, GPA). Use `..Default::default()` for the `None` fields. `description` text is unchanged.
- [ ] **Step 4: Run** `cargo test -p audit-core`. Expected: all pass.
- [ ] **Step 5: Gates, commit** `audit-core: code and credits on missing items`.

### Task 4: Fonts, tokens, pdf.js on demand

**Files:**
- Modify: `vendor/fonts/` — add `anuphan-thai-wght-normal.woff2`, `anuphan-latin-wght-normal.woff2`, `anuphan-latin-ext-wght-normal.woff2` (from `@fontsource-variable/anuphan`), `ibm-plex-mono-latin-400-normal.woff2`, `ibm-plex-mono-latin-500-normal.woff2` (from `@fontsource/ibm-plex-mono`), `LICENSE-anuphan.txt`, `LICENSE-ibm-plex-mono.txt` (OFL texts from those packages); delete the five Geist/Noto woff2 files and their two licence files.
- Modify: `style.css` (`@font-face` blocks, base line-height, print block)
- Modify: `tailwind.config.js` (`fontFamily`, colour tokens)
- Modify: `index.html` (font preloads; pdf.js loading)
- Modify: `sw.js` if it lists font or pdf.js paths in `CORE_ASSETS`

**Interfaces:**
- Produces: Tailwind classes `font-sans` (Anuphan), `font-mono` (IBM Plex Mono); colour tokens `accent-{50..900}` (blue, mockup `--accent` light `#2347c5`, dark `#7d98ff`), keep `brand` zinc; `window.extractTextFromPDF(arrayBuffer)` keeps its signature and error messages.

- [ ] **Step 1: Download the font files** with `npm pack` or the jsDelivr file URLs for the exact package versions into a scratch folder; copy only the listed files into `vendor/fonts`. Record package name and version in each licence file header line.
- [ ] **Step 2: Replace `@font-face` blocks** in `style.css` (same structure as today, `format("woff2-variations")` for Anuphan, unicode ranges from the fontsource CSS of each subset). Add `body { line-height: 1.65 }` in `@layer base`.
- [ ] **Step 3: Update `tailwind.config.js`**: `sans: ["Anuphan", "IBM Plex Sans", "system-ui", "sans-serif"]`, `mono: ["IBM Plex Mono", "ui-monospace", "monospace"]`, add `accent` palette.
- [ ] **Step 4: `index.html`**: add `<link rel="preload" href="fonts/anuphan-thai-wght-normal.woff2" as="font" type="font/woff2" crossorigin>` and the same for the Latin file; remove the `<script src="pdfjs/pdf.min.js" …>` from `<head>`; in `extractTextFromPDF`, before using `pdfjsLib`, await a `loadPdfJs()` that injects that same script once (same `src`, `integrity`, `crossorigin` if present), resolves on `load`, rejects on `error` with the existing message `'PDF.js could not be loaded. Please reload the page.'`, and then sets `pdfjsLib.GlobalWorkerOptions.workerSrc = 'pdfjs/pdf.worker.min.js'`. Delete the now-unused `__pdfjsCdnFailed` flag.
- [ ] **Step 5: Remove Thai letter-spacing**: drop `uppercase` and `tracking-*` from `hero.rs:17,28`, `sidebar.rs:301,315`, `missing_requirements.rs:94` (these files may be rewritten later; this keeps the branch shippable at every commit).
- [ ] **Step 6: Verify** `trunk serve` (or release build + `python -m http.server` in `dist`), open in the browser pane: computed `font-family` of `body` starts with `Anuphan` and `document.fonts.check('16px Anuphan','ทดสอบ')` is true; network list shows `pdf.min.js` absent on load and present after choosing a PDF; uploading a sample PDF still produces results.
- [ ] **Step 7: Gates, commit** `Anuphan and IBM Plex Mono, accent tokens, pdf.js loaded on demand`.

### Task 5: Real result vs plan result

**Files:**
- Modify: `src/app.rs` (`AppState`, the audit `Effect`)
- Modify: `src/components/what_if.rs` (read plan result; hint text)
- Modify: `src/components/offerings.rs:81-82` ("still need" uses real parsed courses only)

**Interfaces:**
- Produces: `AppState.plan_result: ReadSignal<Option<AuditResult>>` (real + planned; `None` when no planned courses). `AppState.audit_result` is real-only from now on.

- [ ] **Step 1: Split the effect**: compute `real = build_audit(&parsed, cur)` and set `audit_result`; save `audit_v1` from `real`; when `planned` is non-empty compute `build_audit(&parsed ++ planned, cur)` into `plan_result`, else set `None`.
- [ ] **Step 2: `what_if.rs`**: replace the "results above include the plan" hint with "the projection below includes the plan" (TH: "ผลคาดการณ์ด้านล่างรวมแผนแล้ว"); the projection strip itself comes in Task 9.
- [ ] **Step 3: `offerings.rs`**: the "only courses I still need" filter uses `parsed_courses` only.
- [ ] **Step 4: Verify in the browser**: add a planned course; Export JSON output and `localStorage.audit_v1` (with save on) do not contain it; the hero total does not change.
- [ ] **Step 5: Gates, commit** `Keep planned courses out of the real audit result`.

### Task 6: Empty state and app shell without the sidebar

**Files:**
- Create: `src/components/empty_state.rs` — moves the upload, analyse, curriculum select and save toggle logic out of `sidebar.rs` unchanged, renders the mockup's empty state, and shows the existing loading and error views in place of the drop zone.
- Delete: `src/components/sidebar.rs` (after its logic is moved)
- Modify: `src/components/header.rs` — sticky 60px header: logo, programme pill (hidden < 860px), file chip with "เปลี่ยนไฟล์ / Change file" when a result exists, EN/TH toggle, theme toggle (existing logic).
- Modify: `src/app.rs` view — one centred column `max-w-[1200px]`; `EmptyState` + `OfferingsPanel` when no result, `ResultsView` otherwise.
- Modify: `src/components/mod.rs`

**Interfaces:**
- Consumes: `AppState` (Task 5).
- Produces: `#[component] pub fn EmptyState() -> impl IntoView`; "Change file" calls one function `reset_to_empty(state: &AppState)` in `empty_state.rs` that clears `audit_result`, `plan_result` source (`planned_courses`), `parsed_courses`, `file_name`, `dropped_file` and the error.

- [ ] **Step 1: Move code**, keeping handler bodies byte-for-byte where possible; restyle markup to the mockup's empty state (headline, explanation, drop zone, curriculum select with an "auto-detect from student ID" first option mapping to today's default behaviour, Analyse button, privacy line, three steps, save checkbox).
- [ ] **Step 2: Header file chip and reset.**
- [ ] **Step 3: Verify** at 375 / 820 / 1280px: no horizontal scroll (`document.documentElement.scrollWidth === innerWidth`); upload → results; "Change file" → empty state with offerings still searchable.
- [ ] **Step 4: Gates, commit** `Empty state replaces the sidebar; header file chip`.

### Task 7: Results shell — head, banner, KPIs, tabs

**Files:**
- Modify: `src/components/results.rs` (page head with Export/Print, status banner, KPI row, tab bar, panels)
- Delete: `src/components/hero.rs`, `src/components/category_summary.rs` (replaced by the KPI row and the Overview)
- Create: `src/components/status.rs` — `pub fn status_line(result: &AuditResult, is_thai: bool) -> (bool, String, Option<String>)` returning (complete, headline, overflow note). Pure; unit-tested.

**Interfaces:**
- Consumes: `AuditResult::{counted_credits, required_credits, earned_credits}`, `Category::{overflow_credits, is_complete}` (Task 2), `MissingCourse.credits` (Task 3).
- Produces: `#[derive(Clone, Copy, PartialEq)] pub enum Tab { Overview, Courses, Semesters, Plan, Offerings }` in `results.rs`, selected tab in a local signal.

- [ ] **Step 1: Tests for `status_line`** (native `cargo test`, in `status.rs`):

```rust
#[test] fn incomplete_with_overflow() { /* 30/30, 87/96, 21/6 */ let (ok, head, note) = status_line(&sample(), true); assert!(!ok); assert!(head.contains('9')); assert!(note.unwrap().contains("15")); }
#[test] fn complete() { /* 30/30, 96/96, 6/6 */ assert!(status_line(&done(), false).0); }
#[test] fn gpa_below_minimum_is_incomplete() { /* all categories full, missing_subjects has category "Graduation" */ assert!(!status_line(&low_gpa(), false).0); }
```

Headline copy (TH / EN): incomplete → "ยังไม่ครบตามหลักสูตร เหลืออีก {n} หน่วยกิต" / "Not complete yet: {n} credits to go"; complete → "ครบตามหลักสูตรแล้ว" / "Curriculum requirements met"; overflow note when `earned > required` and not complete → "หน่วยกิตรวม {earned} เกินเกณฑ์ {required} แล้ว แต่ {overflow} หน่วยกิตที่เกินในหมวดเลือกเสรีนำมานับแทนหมวดอื่นไม่ได้" / "Total {earned} exceeds {required}, but the {overflow} extra free-elective credits can't count toward other categories". `{n}` = `required_credits() - counted_credits()`, formatted without trailing `.0`.
- [ ] **Step 2: Run, expect failure; implement; run, expect pass.**
- [ ] **Step 3: KPI row** per spec: counted credits with `<progress>` (`aria-valuetext` "123 จาก 132 หน่วยกิต"), GPA (or "—" when `None`) with `min_gpa` from `curriculum(&curriculum_id)`, still-to-take (sum of missing credit gaps, categories listed), co-op prep hours read from the same localStorage key `coop_hours_v1` the co-op card uses (factor a `pub fn read_hours() -> Option<(f32, f32)>` out of `coop_hours.rs`). Grid: `grid-cols-2` with first and last `col-span-2`; `min-[700px]:grid-cols-3` with last `col-span-1`; `lg:grid-cols-[1.35fr_1fr_1fr_1fr]` with no spans.
- [ ] **Step 4: Tab bar**: `role="tablist"`, buttons `role="tab"` with `aria-selected`, `aria-controls`, roving `tabindex`, ArrowLeft/ArrowRight; horizontally scrollable with `scrollbar-none`. Registration-mode banner stays above the status banner. Footer line under the panels with the save-results toggle (same signal and storage logic as the empty-state checkbox).
- [ ] **Step 5: Verify** in browser (light/dark, three widths), gates, commit `Results shell: status banner, KPI row, tabs`.

### Task 8: Overview tab

**Files:**
- Create: `src/components/overview.rs` — progress-by-category card, what-to-do-next card, GPA-by-semester chart, and `CoopHours` below.
- Delete: `src/components/missing_requirements.rs` (its content moves into what-to-do-next)

**Interfaces:**
- Consumes: `Category.groups` (Task 2), `MissingCourse.{code, credits}` (Task 3), offerings index fetch from `offerings.rs` (`fetch_json`, make it `pub(crate)`), `compute_gpa`.

- [ ] **Step 1: Category rows**: name with Thai label map (`General Education`→`หมวดศึกษาทั่วไป`, `Major Courses`→`หมวดวิชาเฉพาะ`, `Free Electives`→`หมวดเลือกเสรี`, groups `Basic Science`→`วิทยาศาสตร์พื้นฐาน`, `Core Courses`→`วิชาแกน`, `Capstone`→`โครงงานหรือสหกิจ`, `Major Electives`→`วิชาเลือกเฉพาะ`), tag (ครบ / ขาด n / เกิน +n), `collected / required`, `<progress>`; major groups as indented sub-rows; overflow category shows a split bar (counted solid, overflow hatched) and a legend.
- [ ] **Step 2: What to do next**: one row per `MissingCourse` with `code` (code, name from `description` after `" - "`, credits, next term offered if any term in the offerings index lists the code, link switches to the Offerings tab), then credit-gap rows (`code == None`, `credits == Some`), then rule rows (description as-is). Empty → "ไม่มีรายการค้าง" / "Nothing left".
- [ ] **Step 3: GPA chart**: one bar per semester from `result.attempts` grouped by `semester` using `compute_gpa`, height = gpa / 4.0, last term highlighted, value label above; hidden when fewer than two graded terms.
- [ ] **Step 4: Verify** with the owner's PDF locally (not committed): rows match the old category numbers; gates; commit `Overview tab`.

### Task 9: Courses, Semesters, Plan and Offerings tabs

**Files:**
- Modify: `src/components/category_card.rs` → course table per category inside `<details>` (code, name, term hidden < 600px, credits, grade badge; IP badge amber)
- Modify: `src/components/semester_breakdown.rs` → card grid `grid-cols-1 md:grid-cols-2 xl:grid-cols-3`, header with term, credits, GPA tag; in-progress tag
- Modify: `src/components/what_if.rs` → form row (`md:grid-cols-[150px_1fr_90px_90px_auto]`), planned table, projection strip from `plan_result` (counted credits, projected GPA, status tag via `status_line`)
- Modify: `src/components/offerings.rs` → toolbar (term select, search, checkbox), table styling; the checkbox renders only when a result exists

**Interfaces:**
- Consumes: `plan_result` (Task 5), `status_line` (Task 7), `Tab` (Task 7).

- [ ] **Step 1–4: One sub-step per component**, restyle to the mockup, keep all existing behaviour (prerequisite warnings, retake handling, term picker, search).
- [ ] **Step 5: Verify** each tab at three widths and in English; no horizontal page scroll (tables may scroll inside `.overflow-x-auto`).
- [ ] **Step 6: Gates, commit** `Courses, Semesters, Plan and Offerings tabs`.

### Task 10: Print, final checks, PR

**Files:**
- Modify: `style.css` print block (show every tab panel, hide header, tab bar and actions; `break-inside: avoid` on cards)
- Modify: `README.md` only where it describes the UI layout or fonts

- [ ] **Step 1: Print view**: Print preview shows banner, KPIs, all five panels in order.
- [ ] **Step 2: Full browser pass** at 375, 820, 1280px, light and dark, TH and EN: scrollWidth check, keyboard through tabs, focus rings visible; registration-mode PDF shows GPA "—"; planned course absent from Export JSON.
- [ ] **Step 3: Owner's TH and EN transcripts** (local only): same course count (55), category credits, GPA as on `main`.
- [ ] **Step 4: Release build size**: record wasm gzip and total transfer of first load.
- [ ] **Step 5: Gates, push `ui-redesign`, open PR** with the before/after sizes and screenshots at three widths; wait for CI green; ask the owner to merge.
