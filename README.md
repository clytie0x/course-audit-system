# Course Audit System

> Privacy-first transcript auditor for **PSU Computer Science** — powered by **Rust + WebAssembly**.

Upload your PDF transcript (or registration report), and see exactly where you stand against graduation requirements. **Everything runs in your browser — no server, no uploads, no data leaves your device.**

---

## Features

- **Audit engine** — matches courses against the full PSU CS curriculum: General Education (30 cr), Basic Science, Core, Capstone (choose one), Major Electives (cluster rules), and auto-detected Free Electives.
- **Two PDF formats** — accepts both the **SIS Grade Report** (with grades) and the **Registration Report** (no grades yet). The parser auto-detects which and flags in-progress courses.
- **GPA calculator** — overall GPA on a 4.0 scale, weighted by credits, excluding W/P/S/G/IP.
- **What-If simulator** — add hypothetical courses with expected grades and see the projected impact on credits and GPA live.
- **Semester view** — drill into each `X/YYYY` term to see courses and per-semester GPA.
- **Dark mode** — system-preference aware, toggleable, flash-free on first paint.
- **Export & share** — JSON download, Print / Save-PDF, copy missing requirements to clipboard.
- **Offline-capable (PWA)** — installable; app shell cached via service worker.
- **Bilingual** — ไทย / English toggle, persists across sessions.
- **Opt-in save** — optionally keep the last result in `localStorage`; off by default.

---

## Quick Start

### Prerequisites

| Tool                         | Version | Purpose                                    |
| ---------------------------- | ------- | ------------------------------------------ |
| [Rust](https://rustup.rs)    | stable  | Compiler + `wasm32-unknown-unknown` target |
| [Trunk](https://trunkrs.dev) | 0.17+   | WASM bundler, dev server, Tailwind CLI     |

```bash
# one-time
rustup target add wasm32-unknown-unknown
cargo install trunk
```

### Development

```bash
trunk serve --open
# → http://localhost:8080
```

### Production Build

```bash
trunk build --release
# → dist/   (opt-level='z', LTO, single codegen-unit)
```

---

## How It Works

```
 PDF  ──▶  PDF.js       ──▶  Rust parser   ──▶  Audit engine  ──▶  Leptos UI
          (browser)         (auto-detect        (GenEd / Major      (donuts,
                             Grade vs            / electives /       progress,
                             Registration)       free electives      What-If,
                                                 + GPA)              semesters)
```

1. **Upload** — drag & drop or click. PDF never leaves the browser.
2. **Extract** — PDF.js pulls raw text per page (JS interop from WASM).
3. **Parse** — a regex-based parser runs in Rust:
   - Two patterns, tried in order: Grade-report (`code name sec credit grade`) → Registration (`code name sec category credit-float`).
   - Semester headers (`ภาคการศึกษา X/YYYY`) anchor every course to its term.
   - Course codes are normalized (`890-001 → 890-101`, trailing `G1/G4/G2A…` stripped).
   - Repeated special topics (`344-496`…`344-499`) get greedy `(Topic N)` suffixes.
4. **Audit** — `build_audit()` combines user's courses (+ any What-If planned ones) with curriculum data:
   - **GenEd** supports 4 selection rules: `choose_all`, `choose_one`, `choose_sequential_pair`, `choose_all_sub_groups`.
   - **Major** enforces exact-slot requirements for Basic Science / Core / Capstone, plus cluster-completion rules for electives.
   - **In-progress (IP)** courses fill curriculum slots but contribute **0 credits** until graded.
5. **Display** — Leptos renders donut charts, progress bars, a per-semester accordion, missing requirements, and the What-If planner. GPA shown beside total credits.

### Privacy guarantees

| Layer          | Behavior                                                    |
| -------------- | ----------------------------------------------------------- |
| **Processing** | 100% in-browser WebAssembly                                 |
| **Network**    | Only loads static assets + PDF.js CDN; no user data sent    |
| **Storage**    | Nothing by default. Opt-in `localStorage` for last result   |
| **Code**       | Open source, fully auditable                                |

---

## Project Structure

```
course-audit-system/
├── Cargo.toml                    # Rust deps + release profile (opt-level='z', LTO)
├── Cargo.lock                    # Tracked (binary crate)
├── Trunk.toml                    # Bundler config
├── index.html                    # Entry + PDF.js loader + service-worker registration
├── manifest.webmanifest          # PWA manifest (standalone, theme colors)
├── sw.js                         # Service worker (app-shell cache, network-first)
├── style.css                     # Tailwind directives + print stylesheet
├── tailwind.config.js            # darkMode: 'class', brand palette, animations
│
├── src/
│   ├── main.rs                   # Tiny entry point — panic hook + mount App
│   ├── app.rs                    # Top-level App component, all UI state
│   ├── models.rs                 # Data types + utility fns (GPA, dedupe, IP, etc.)
│   │
│   ├── components/
│   │   └── category_card.rs      # Collapsible category accordion
│   │
│   ├── data/
│   │   ├── gen_ed.rs  + gen_ed.json
│   │   └── major.rs   + major.json   # JSON is the source of truth; .rs loads it
│   │
│   └── logic/
│       ├── parser.rs             # Two regex patterns + semester detection
│       └── auditor.rs            # GenEd/Major/FreeElective + build_audit()
│
└── .github/
    ├── workflows/
    │   ├── ci.yml                # fmt · clippy · test on every push/PR
    │   └── deploy.yml            # Build with Trunk → GitHub Pages
    └── dependabot.yml            # Weekly cargo + monthly actions updates
```

### Editing the curriculum

The two JSON files under `src/data/` are the source of truth. Edit them directly — no Rust knowledge needed. If you'd rather modify the Rust form and dump JSON, there's a regen helper:

```bash
# After editing the get_*_curriculum() builders, regenerate the JSON:
cargo test dump_curricula -- --ignored
```

---

## Curriculum Coverage

| Category              | Required    | Details                                                                                   |
| --------------------- | ----------- | ----------------------------------------------------------------------------------------- |
| **General Education** | 30 cr       | 7 strands (choose-all / choose-one / choose-sequential-pair / sub-groups) + 6 elective sub-categories |
| **Basic Science**     | 12 cr       | Math, Physics, Chemistry foundations                                                      |
| **Core Courses**      | 56 cr       | All required CS courses                                                                   |
| **Capstone**          | 3–6 cr      | Senior Project *or* Co-operative Education (choose 1)                                     |
| **Major Electives**   | 12 cr       | Domain-based clusters — complete at least 2 full clusters                                 |
| **Free Electives**    | 6 cr        | Auto-detected from unmatched passing courses                                              |
| **Total**             | **~132 cr** | Full B.Sc. (Computer Science)                                                             |

### Audit rules in detail

- **Slot-fill vs credit count.** A passing grade fills its slot *and* adds credits. An **IP** course fills its slot but adds **0 credits** — it shows as "in progress" and doesn't unfairly fail the audit.
- **Greedy matching.** Repeatable special topics (344-496…499) accumulate across multiple enrollments.
- **Code normalization.** Legacy codes (`890-001 → 890-101`, etc.) and suffixed codes (`890-103G1 → 890-103`) are mapped to the curriculum form.
- **Credit cap.** Credits come from the curriculum definition, capped by the transcript value — this guards against PDF-parse drift inflating a course's credit.
- **Deduplication.** Special topics keyed by `code::name`; regular courses by code.

---

## Tech Stack

| Layer     | Technology                                | Role                                   |
| --------- | ----------------------------------------- | -------------------------------------- |
| Language  | **Rust** (2021 edition)                   | Type-safe, compiles to WASM            |
| UI        | **Leptos 0.6** (CSR)                      | Reactive UI with fine-grained signals  |
| Build     | **Trunk**                                 | WASM bundler, asset pipeline, dev server |
| Styling   | **Tailwind CSS** (class-based dark mode)  | Utility-first                          |
| Fonts     | **Inter Variable**, **Prompt**, **JetBrains Mono** | UI · Thai · monospace            |
| PDF       | **PDF.js 3.11**                           | Client-side text extraction            |
| Interop   | `wasm-bindgen`, `web-sys`, `js-sys`, `gloo-file` | Rust ↔ JavaScript bridge      |
| Persist.  | `localStorage`, `serde_json`              | Opt-in save/restore                    |

### Release optimizations

```toml
[profile.release]
opt-level = 'z'    # size over speed
lto = true
codegen-units = 1
```

---

## Quality Gates

- `cargo fmt --all -- --check` — formatting
- `cargo clippy --all-targets -- -D warnings` — lints as errors
- `cargo test` — unit tests (26 covering parser, auditor, GPA, IP handling, semester tagging, legacy normalization, special-topic greedy match)

All three run on every push and PR via `.github/workflows/ci.yml`. A Rust build cache keeps CI fast.

---

## Deployment

### GitHub Pages (automatic)

Push to `main` → `.github/workflows/deploy.yml` builds with Trunk and deploys to Pages.

### Manual / other hosts

```bash
trunk build --release --public-url /your-base-path/
# Upload dist/ to any static host (Netlify, Vercel, Cloudflare Pages, S3…)
```

---

## License

MIT
