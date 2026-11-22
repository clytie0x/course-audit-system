<div align="center">

# Course Audit System

**Privacy-first transcript auditor for PSU Computer Science**

Upload a PDF — see exactly where you stand on graduation requirements.
Everything runs in your browser. Nothing ever leaves your device.

<br />

[![CI](https://github.com/clytie0x/course-audit-system/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/clytie0x/course-audit-system/actions/workflows/ci.yml)
[![Deploy](https://github.com/clytie0x/course-audit-system/actions/workflows/deploy.yml/badge.svg?branch=main)](https://github.com/clytie0x/course-audit-system/actions/workflows/deploy.yml)
![Rust](https://img.shields.io/badge/rust-stable-orange?logo=rust&logoColor=white)
![Leptos 0.8](https://img.shields.io/badge/leptos-0.8-purple)
![WASM](https://img.shields.io/badge/wasm-WebAssembly-654ff0?logo=webassembly&logoColor=white)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](#license)

[**Quick start**](#quick-start) · [**Features**](#features) · [**How it works**](#how-it-works) · [**Project structure**](#project-structure) · [**Curriculum**](#curriculum-coverage)

</div>

---

## Features

<table>
<tr>
<td width="50%" valign="top">

### Full audit engine
Matches courses against the PSU CS curriculum revision for your cohort (2564 for รหัส 64–68, 2569 for รหัส 69+), picked from the student ID and switchable in the sidebar.

</td>
<td width="50%" valign="top">

### Two PDF formats, auto-detected
**SIS Grade Report** (with grades) and **Registration Report** (no grades yet). In-progress courses fill their curriculum slot but contribute 0 credits until graded.

</td>
</tr>
<tr>
<td valign="top">

### GPA calculator
Overall GPA on the 4.0 scale, weighted by credit hours, over every graded attempt (E/F and retakes included), excluding W/P/S/U/G/IP.

</td>
<td valign="top">

### What-If simulator
Add hypothetical courses with expected grades, watch credits and GPA update live.

</td>
</tr>
<tr>
<td valign="top">

### Semester-by-semester view
Collapsible accordion for each `X/YYYY` term with per-semester GPA and credit totals.

</td>
<td valign="top">

### Export & share
JSON download · Print / Save-PDF (print stylesheet) · Copy missing requirements to clipboard.

</td>
</tr>
<tr>
<td valign="top">

### Dark mode
System-preference aware, toggleable, flash-free on first paint (pre-hydration script).

</td>
<td valign="top">

### Offline-capable PWA
Installable, app shell cached via service worker, works without network on repeat visits.

</td>
</tr>
<tr>
<td valign="top">

### Bilingual
ไทย / English toggle, persisted across sessions, separate from system language.

</td>
<td valign="top">

### Opt-in persistence
Off by default. Optional checkbox keeps your last result in `localStorage` for next time.

</td>
</tr>
</table>

---

## Quick start

**Prerequisites**

| Tool | Version | Purpose |
| --- | --- | --- |
| [Rust](https://rustup.rs) | stable | Compiler + `wasm32-unknown-unknown` target |
| [Trunk](https://trunkrs.dev) | 0.22+ | WASM bundler · dev server · Tailwind CLI (pinned to v3.4.17 in Trunk.toml) |

**One-time setup**

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
```

**Develop**

```bash
trunk serve --open
# → http://localhost:8080  (hot reload)
```

**Release build**

```bash
trunk build --release
# → dist/  (opt-level='z', LTO, single codegen-unit)
```

---

## How it works

```
┌──────────────┐   ┌──────────────┐   ┌──────────────┐   ┌──────────────┐
│  Upload PDF  │ → │   PDF.js     │ → │ Rust parser  │ → │ Audit engine │
│ (drag/drop)  │   │ (browser,    │   │ (regex,      │   │ (GenEd +     │
│              │   │  JS interop) │   │  auto-detect │   │  Major +     │
│              │   │              │   │  format)     │   │  electives + │
│              │   │              │   │              │   │  GPA)        │
└──────────────┘   └──────────────┘   └──────────────┘   └──────┬───────┘
                                                                │
                                                         ┌──────▼───────┐
                                                         │  Leptos UI   │
                                                         │ donuts · bars│
                                                         │ What-If      │
                                                         └──────────────┘
```

1. **Upload** — drag & drop or click. The file is read into memory only.
2. **Extract** — PDF.js pulls raw text per page via JS interop from WASM.
3. **Parse** — a regex-based parser (two patterns tried in order):
   - Grade-report: `code · name · section · credit · grade`
   - Registration: `code · name · section · category · credit-float` (tagged as IP)
   - Semester headers (`ภาคการศึกษา X/YYYY`, or `Semester X/YYYY` with the year converted to B.E.) anchor every course to its term.
   - Course codes are normalized (`890-001 → 890-101`); the trailing GE category (`G1/G4/G2A…`) is kept as the course's tag.
   - The cohort is read from the student ID to pick the curriculum revision.
   - Repeated special topics (`344-496…499`) get greedy `(Topic N)` suffixes.
4. **Audit** — `build_audit()` combines parsed + any What-If planned courses and runs:
   - **GenEd**: 5 rules — `choose_all`, `choose_one`, `choose_sequential_pair`, `choose_all_sub_groups`, `by_tag` (GE category from the code suffix).
   - **Major**: exact-slot matching for Basic Science / Core / Capstone; electives by cluster completion (2564) or 12 credits in one domain (2569); non-credit courses that must be passed (344-494).
   - **Overflow**: GenEd and major-elective credits beyond the requirement count as free electives.
   - **In-progress (IP)**: fills the slot, adds **0 credits** until graded.
5. **Display** — Leptos renders donut charts, progress bars, per-semester accordion, missing requirements, What-If planner. GPA shown next to total credits.

### Privacy guarantees

| Layer | Behavior |
| --- | --- |
| **Processing** | 100% in-browser WebAssembly |
| **Network** | Same-origin static assets (PDF.js is bundled) — no user data ever sent |
| **Storage** | Nothing by default. Opt-in `localStorage` for the last result, its parsed courses (so What-If and curriculum changes re-audit after a reload) and co-op hours |
| **Code** | Open source, fully auditable |

---

## Project structure

```
course-audit-system/
├── Cargo.toml               Rust deps + release profile
├── Trunk.toml               Bundler config
├── index.html               Entry + PDF.js loader + SW registration
├── manifest.webmanifest     PWA manifest (standalone, theme colors)
├── sw.js                    Service worker (app-shell cache, network-first)
├── style.css                Tailwind directives + print stylesheet
├── tailwind.config.js       darkMode: 'class' + brand palette + animations
│
├── src/                     Web crate (Leptos UI, wasm only)
│   ├── main.rs              Tiny entry — panic hook + mount App
│   ├── app.rs               App shell · shared AppState signals · re-audit effect
│   ├── pdf.rs               PDF.js interop (extractTextFromPDF)
│   └── components/
│       ├── header.rs        Navbar · language + dark-mode toggles
│       ├── sidebar.rs       Upload + analysis · how-it-works · save toggle
│       ├── results.rs       Results layout · export JSON / print
│       ├── hero.rs          Total credits + GPA
│       ├── category_summary.rs  Donut card per category
│       ├── category_card.rs Collapsible category accordion
│       ├── what_if.rs       What-If simulator
│       ├── semester_breakdown.rs  Per-semester GPA + credits
│       ├── missing_requirements.rs  Missing list + copy
│       └── offerings.rs     SIS offerings search + personal timetable
│
├── crates/audit-core/       Pure Rust core, no wasm deps (tests run natively)
│   └── src/
│       ├── models.rs        Data types + utilities (GPA, dedupe, IP)
│       ├── parser.rs        Two regex patterns + semester detection
│       ├── auditor.rs       GenEd / Major / FreeElective + build_audit()
│       ├── offerings.rs     Offering types, search, clash detection
│       └── data/
│           ├── mod.rs                    Registry: curricula(), curriculum_for_cohort()
│           ├── cs2564.json               หลักสูตร 2564 (JSON is source of truth)
│           ├── cs2569.json               หลักสูตร 2569
│           └── SOURCES.md                Official documents, URLs, SHA-256
│
├── data/offerings/          SIS course offerings per term (JSON, scraped)
└── tools/sis-scraper/       Playwright scraper for data/offerings
```

<details>
<summary><b>Editing the curriculum</b></summary>

Each revision is one JSON file under `crates/audit-core/src/data/` (`cs2564.json`, `cs2569.json`), transcribed from the official มคอ.2 on [sci.psu.ac.th](https://www.sci.psu.ac.th/academic-programs/bachelors-degree/bachelor-of-science-computer-science/). Edit them directly — no Rust knowledge needed. `cargo test -p audit-core` checks that each file's credit totals match its มคอ.2. [`SOURCES.md`](crates/audit-core/src/data/SOURCES.md) lists each source document with its URL and SHA-256.

</details>

---

## Curriculum coverage

| Category | 2564 (รหัส 64–68) | 2569 (รหัส 69+) |
| --- | --- | --- |
| **General Education** | 30 cr — 7 strands + 6 elective (accepts both the cohort 64–66 and 67–68 course sets) | 24 cr — GE1–GE7 (18) + GE8 (6, at least 2 of them a language: 848-/890-/891-), matched by GE tag |
| **Basic / core science** | 12 cr — chemistry, biology, physics | 12 cr — 315-101, 322-101, 345-101, 346-101 |
| **Required major** | 48 cr + capstone: project 344-492 (3) or co-op 344-495 (6, needs 344-494) | 56 cr incl. project 344-491 and co-op 344-492; 344-494 must be passed |
| **Major electives** | 33 cr (project plan) or 30 cr (co-op plan); complete 2 clusters | 27 cr; at least 12 cr in one domain |
| **Free electives** | 6 cr | 6 cr |
| **Total** | **132 cr**, GPA ≥ 2.00 | **125 cr**, GPA ≥ 2.00 |

<details>
<summary><b>Audit rules in detail</b></summary>

- **Slot-fill vs credit count.** A passing grade fills its slot *and* adds credits. An **IP** (in-progress) course fills its slot but adds **0 credits** — the audit doesn't unfairly fail you for current-semester courses.
- **Greedy matching.** Repeatable special topics (344-496…499 in 2564, 344-495…498 in 2569) accumulate across multiple enrollments.
- **Prerequisites.** Each revision lists its "รายวิชาบังคับเรียนก่อน" from the มคอ.2; the What-If planner flags a planned course whose prerequisite has not been taken (any grade, or in progress, counts; W does not).
- **Same code, different course.** Codes change meaning between revisions (344-491 is the seminar in 2564 but the project in 2569), so every check uses the selected revision's own list.
- **Code normalization.** Legacy codes (`890-001 → 890-101`) and suffixed codes (`890-103G1 → 890-103`, tag `G1`) are mapped to the curriculum form.
- **Credit cap.** Credits come from the curriculum definition, capped by the transcript value — guards against PDF-parse drift inflating a course's credit.
- **Deduplication.** Special topics are keyed by `code::name`; regular courses by code alone.

</details>

---

## Tech stack

<table>
<tr>
<td><b>Language</b></td>
<td>Rust (2021 edition) — compiles to WASM</td>
</tr>
<tr>
<td><b>UI</b></td>
<td>Leptos 0.8 (CSR) — reactive UI with fine-grained signals</td>
</tr>
<tr>
<td><b>Build</b></td>
<td>Trunk — WASM bundler, asset pipeline, dev server</td>
</tr>
<tr>
<td><b>Styling</b></td>
<td>Tailwind CSS v3 (class-based dark mode), zinc monochrome in the style of shadcn/ui</td>
</tr>
<tr>
<td><b>Fonts</b></td>
<td>Geist · Noto Sans Thai · Geist Mono, self-hosted in <code>vendor/fonts</code> (SIL OFL)</td>
</tr>
<tr>
<td><b>PDF</b></td>
<td>PDF.js 3.11 — client-side text extraction</td>
</tr>
<tr>
<td><b>Interop</b></td>
<td><code>wasm-bindgen</code> · <code>web-sys</code> · <code>js-sys</code> · <code>gloo-file</code></td>
</tr>
<tr>
<td><b>Persistence</b></td>
<td><code>localStorage</code> + <code>serde_json</code> (opt-in)</td>
</tr>
</table>

### Release optimizations

```toml
[profile.release]
opt-level = 'z'      # size over speed
lto = true
codegen-units = 1
```

---

## Quality gates

Every push and pull request runs `.github/workflows/ci.yml`. The same gates run locally:

```
cargo fmt --all -- --check                                format
cargo clippy --workspace --all-targets -- -D warnings     lints as errors (native)
cargo clippy --target wasm32-unknown-unknown -- -D warnings   lints as errors (wasm)
cargo test --workspace --all-targets                      64 unit tests
trunk build --release --public-url /course-audit-system/  release build
```

CI also syntax-checks the scraper and fails if `data/offerings` contains a 10-digit number, a passcode or a Zoom meeting ID.

Test coverage spans the parser (format auto-detect, code normalization, Thai and English semester headers, GE tags, cohort detection, student IDs not mistaken for course codes), the auditor (GenEd rules incl. by-tag, IP slot-fill vs credits, plan-dependent elective targets, domain rule, non-credit requirements, overflow to free electives, GPA minimum), curriculum data (credit totals match each มคอ.2), and utilities (GPA over every graded attempt, in-progress detection).

---

## Course offerings (SIS)

`data/offerings/<year>-<term>.json` holds the public course offerings of the Hat Yai campus, as listed by SIS: sections, seats, target students, class times, rooms, instructors and exam dates. Trunk copies the folder to `dist/offerings/`, and the **รายวิชาที่เปิดสอน** panel loads it. In that panel you can:

- search by code or name
- filter to required courses you have not taken yet
- see prerequisite warnings
- pick sections into a personal timetable, which flags class-time and exam clashes

**Co-op preparation hours.** The co-op course needs 30 preparation hours: at least 24 from faculty or programme activities and at least 6 from university or other activities. The Faculty of Science publishes these in Looker Studio reports at [regis.sci.psu.ac.th/activity30hr](https://regis.sci.psu.ac.th/activity30hr), one per cohort.

A static site cannot query those reports: their data API needs a per-session XSRF token and sends no CORS headers. The co-op card works around this in two ways:

- It embeds the official report for the cohort read from your student ID, so you can search your ID inside it.
- It takes the two totals by hand.

`node tools/sis-scraper/coop-hours.mjs <student id>` filters a report to one ID and prints only that student's rows and totals.

**Automatic refresh.** `.github/workflows/offerings.yml` runs every Monday at 03:00 Thailand time, and can also be started by hand:

- It scrapes the two newest terms SIS lists, so a new term is picked up without any code change.
- It commits `data/offerings` only when something changed.
- Deploy then republishes the site.

The job runs on a self-hosted runner, because SIS is behind Cloudflare and GitHub-hosted runners get HTTP 403 ("Sorry, you have been blocked"). Any machine on a normal Thai connection with Google Chrome, Node and Git Bash works. One-time setup:

1. Settings → Actions → Runners → New self-hosted runner, pick the OS and follow GitHub's download and `config` commands (the token comes from that page).
2. Start it with `run.cmd` (or install it as a service under your own account). Jobs wait up to 24 hours for the runner to come online.
3. Settings → Actions → General → "Approval for running fork pull request workflows from contributors" → **Require approval for all external contributors**. This is a public repo, and a pull request from a fork could otherwise edit a workflow to run on your machine.

Refresh the data by hand with the scraper. SIS is a Blazor Server app, so the scraper uses `playwright-core` to drive the installed Google Chrome. It visits only public pages, one request at a time with a delay:

```bash
cd tools/sis-scraper
npm ci
node scrape.mjs                          # every term SIS lists: 344- plus the curricula's science courses
node scrape.mjs --latest 2 --ge          # what the weekly job runs
node scrape.mjs --terms 2/2569 --ge      # also GenEd course prefixes for one term
node scrape.mjs --terms 2/2569 --keywords 344-,322-101
```

---

## Deployment

`.github/workflows/deploy.yml` builds with Trunk and publishes to GitHub Pages (<https://clytie0x.github.io/course-audit-system/>). It runs on every push to `main`, after each offerings refresh, and on demand. For another host:

```bash
trunk build --release --public-url /your-base-path/
# upload dist/ to Netlify · Vercel · Cloudflare Pages · S3 · etc.
```

---

## License

[MIT](LICENSE)
