<div align="center">

# Course Audit System

**Privacy-first transcript auditor for PSU Computer Science**

Upload a PDF — see exactly where you stand on graduation requirements.
Everything runs in your browser. Nothing ever leaves your device.

<br />

![Rust](https://img.shields.io/badge/rust-stable-orange?logo=rust&logoColor=white)
![Leptos 0.6](https://img.shields.io/badge/leptos-0.6-purple)
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
Matches courses against the PSU CS curriculum:
GenEd (30) · Basic Science (12) · Core (56) · Capstone (3–6) · Major Electives with cluster rules · Free Electives auto-detected.

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
| [Trunk](https://trunkrs.dev) | 0.17+ | WASM bundler · dev server · Tailwind CLI |

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
   - Semester headers (`ภาคการศึกษา X/YYYY`) anchor every course to its term.
   - Course codes are normalized (`890-001 → 890-101`, trailing `G1/G4/G2A…` stripped).
   - Repeated special topics (`344-496…499`) get greedy `(Topic N)` suffixes.
4. **Audit** — `build_audit()` combines parsed + any What-If planned courses and runs:
   - **GenEd**: 4 rules — `choose_all`, `choose_one`, `choose_sequential_pair`, `choose_all_sub_groups`.
   - **Major**: exact-slot matching for Basic Science / Core / Capstone, cluster-completion for electives.
   - **In-progress (IP)**: fills the slot, adds **0 credits** until graded.
5. **Display** — Leptos renders donut charts, progress bars, per-semester accordion, missing requirements, What-If planner. GPA shown next to total credits.

### Privacy guarantees

| Layer | Behavior |
| --- | --- |
| **Processing** | 100% in-browser WebAssembly |
| **Network** | Same-origin static assets (PDF.js is bundled) — no user data ever sent |
| **Storage** | Nothing by default. Opt-in `localStorage` for last result |
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
│   ├── app.rs               App component · all UI state & handlers
│   ├── pdf.rs               PDF.js interop (extractTextFromPDF)
│   └── components/
│       └── category_card.rs Collapsible category accordion
│
└── crates/audit-core/       Pure Rust core, no wasm deps (tests run natively)
    └── src/
        ├── models.rs        Data types + utilities (GPA, dedupe, IP)
        ├── parser.rs        Two regex patterns + semester detection
        ├── auditor.rs       GenEd / Major / FreeElective + build_audit()
        └── data/
            ├── gen_ed.rs + gen_ed.json   JSON is source of truth
            └── major.rs  + major.json    .rs loads via include_str!
```

<details>
<summary><b>Editing the curriculum</b></summary>

The JSON files under `crates/audit-core/src/data/` are the source of truth. Edit them directly — no Rust knowledge needed.

If you'd rather tweak the Rust builders and regenerate JSON:

```bash
cargo test -p audit-core dump_curricula -- --ignored
```

</details>

---

## Curriculum coverage

| Category | Required | Details |
| --- | --- | --- |
| **General Education** | 30 cr | 7 strands (choose-all · choose-one · choose-sequential-pair · sub-groups) + 6 elective sub-categories |
| **Basic Science** | 12 cr | Math · Physics · Chemistry foundations |
| **Core Courses** | 56 cr | All required CS courses |
| **Capstone** | 3–6 cr | Senior Project **or** Co-operative Education — choose 1 |
| **Major Electives** | 12 cr | Domain-based clusters — complete at least 2 full clusters |
| **Free Electives** | 6 cr | Auto-detected from unmatched passing courses |
| **Total** | **~132 cr** | Full B.Sc. (Computer Science) |

<details>
<summary><b>Audit rules in detail</b></summary>

- **Slot-fill vs credit count.** A passing grade fills its slot *and* adds credits. An **IP** (in-progress) course fills its slot but adds **0 credits** — the audit doesn't unfairly fail you for current-semester courses.
- **Greedy matching.** Repeatable special topics (344-496…499) accumulate across multiple enrollments.
- **Code normalization.** Legacy codes (`890-001 → 890-101`) and suffixed codes (`890-103G1 → 890-103`) are mapped to the curriculum form.
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
<td>Leptos 0.6 (CSR) — reactive UI with fine-grained signals</td>
</tr>
<tr>
<td><b>Build</b></td>
<td>Trunk — WASM bundler, asset pipeline, dev server</td>
</tr>
<tr>
<td><b>Styling</b></td>
<td>Tailwind CSS (class-based dark mode) — utility-first</td>
</tr>
<tr>
<td><b>Fonts</b></td>
<td>Inter Variable · Prompt (Thai) · JetBrains Mono</td>
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

CI is paused while the project is experimental. Run these three gates locally before every commit:

```
cargo fmt --all -- --check                       format
cargo clippy --workspace --all-targets -D warnings  lints as errors
cargo test --workspace --all-targets             35 unit tests
```

Test coverage spans the parser (format auto-detect, code normalization, semester tagging, special-topic greedy match), the auditor (GenEd rules, IP slot-fill vs credits, free-elective bucket, retakes counted once, GenEd-elective overflow, credit-shortfall reporting), and utilities (GPA over every graded attempt, in-progress detection).

---

## Deployment

Automatic GitHub Pages deploys are paused while the project is experimental. Build and upload manually:

```bash
trunk build --release --public-url /your-base-path/
# upload dist/ to Netlify · Vercel · Cloudflare Pages · S3 · etc.
```

---

## License

[MIT](LICENSE)
