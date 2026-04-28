<div align="center">

# Course Audit System

**Privacy-first transcript auditor for PSU Computer Science**

Upload a PDF — see exactly where you stand on graduation requirements.
Everything runs in your browser. Nothing ever leaves your device.

<br />

[![CI](https://github.com/Teerapat-Vatpitak/course-audit-system/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/Teerapat-Vatpitak/course-audit-system/actions/workflows/ci.yml)
[![Deploy](https://github.com/Teerapat-Vatpitak/course-audit-system/actions/workflows/deploy.yml/badge.svg?branch=main)](https://github.com/Teerapat-Vatpitak/course-audit-system/actions/workflows/deploy.yml)
![SvelteKit 2](https://img.shields.io/badge/SvelteKit-2-ff3e00?logo=svelte&logoColor=white)
![Svelte 5](https://img.shields.io/badge/Svelte-5%20Runes-ff3e00?logo=svelte&logoColor=white)
![TypeScript](https://img.shields.io/badge/TypeScript-5-3178c6?logo=typescript&logoColor=white)
![Tailwind v4](https://img.shields.io/badge/Tailwind-v4-06b6d4?logo=tailwindcss&logoColor=white)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](#license)

[**Quick start**](#quick-start) · [**Features**](#features) · [**How it works**](#how-it-works) · [**Project structure**](#project-structure)

</div>

---

## Features

- **PDF transcript audit** — drag and drop a PSU SIS transcript (Grade Report or Registration Report) and get an instant, categorised audit
- **Auto-detect format** — automatically distinguishes graded transcripts from registration reports; in-progress (`IP`) courses fill curriculum slots without inflating credit totals
- **GPA calculation** — weighted 4.0 scale, excludes W / P / S / U / G / IP
- **What-if simulator** — add hypothetical courses live and see how your projected audit changes
- **Semester breakdown** — per-term GPA and credit summary, expandable
- **Missing requirements** — grouped by category with copy-to-clipboard
- **Export** — JSON download or print-to-PDF stylesheet
- **Dark mode** — pre-hydration script to avoid flash, persisted across sessions
- **Bilingual** — Thai / English toggle, persisted independently
- **Offline-capable PWA** — service worker uses a network-first, same-origin policy and never caches your transcript
- **Opt-in persistence** — results are stored in `localStorage` only when you explicitly enable the toggle

---

## Quick start

Prerequisites: Node 22+ and pnpm 10+.

```bash
pnpm install
pnpm dev          # http://localhost:5173
```

Other commands:

```bash
pnpm build        # production build to ./build (static, ready for GitHub Pages)
pnpm preview      # preview the production build locally
pnpm test         # run Vitest unit tests
pnpm check        # svelte-check + tsc (no errors expected)
pnpm lint         # ESLint + Prettier
pnpm format       # auto-format the workspace
```

---

## How it works

```
┌──────────────────────┐    ┌──────────────────┐    ┌──────────────────┐    ┌──────────────────────┐
│  PDF (drag/drop)     │ →  │  pdfjs-dist      │ →  │  parser.ts       │ →  │  auditor.ts          │
│  File or input       │    │  (npm, dynamic)  │    │  regex extract   │    │  curriculum match    │
└──────────────────────┘    └──────────────────┘    └──────────────────┘    └──────────────────────┘
                                                                                       │
                                                                                       ▼
                                                                            ┌──────────────────────┐
                                                                            │  Svelte 5 Runes UI   │
                                                                            │  (re-runs on plan ∆) │
                                                                            └──────────────────────┘
```

1. The user picks a PDF. `pdfjs-dist` is dynamically imported only at this point, so the initial bundle stays small.
2. `parseTranscript` runs the text through two regex variants and tags each course with its semester header, normalising codes (`890-001` → `890-101`, `344-101G1` → `344-101`).
3. `buildAudit` matches courses against the GenEd and Major curricula, computes credits per category, and produces a UI-facing `AuditResult`.
4. Svelte 5 stores re-run the audit reactively whenever the parsed transcript or What-If plan changes.

### Privacy guarantees

| Concern                    | Implementation                                                                                                      |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| Where does the PDF go?     | Stays in memory in the browser tab. Never uploaded.                                                                 |
| What gets cached?          | The Service Worker caches only same-origin app shell assets. PDFs are never written to a cache or to disk.          |
| What about `localStorage`? | Only when you tick **Save results in browser**. The toggle is off by default; turning it off clears any saved data. |

---

## Project structure

```
course-audit-system/
├── src/
│   ├── app.html, app.css, app.d.ts
│   ├── hooks.client.ts                     (theme guard, optional)
│   ├── service-worker.ts                   (network-first, same-origin)
│   ├── routes/
│   │   ├── +layout.svelte / +layout.ts     (CSR-only, prerendered)
│   │   └── +page.svelte                    (main audit screen)
│   └── lib/
│       ├── audit/
│       │   ├── models.ts                   (Course, AuditResult, GenEd/Major curricula, helpers)
│       │   ├── parser.ts                   (PDF text → ParsedCourse[])
│       │   └── auditor.ts                  (GenEd + Major + Free Electives + GPA)
│       ├── data/
│       │   ├── gen_ed.json, major.json     (curriculum, snake_case verbatim)
│       │   └── curricula.ts                (typed loaders)
│       ├── pdf/extract.ts                  (pdfjs-dist dynamic import)
│       ├── stores/
│       │   ├── auditStore.svelte.ts        (parsed + planned + result)
│       │   ├── settingsStore.svelte.ts     (lang, theme, save toggle)
│       │   └── persistence.ts              (versioned localStorage adapter)
│       ├── i18n/index.ts                   (Thai / English catalog)
│       ├── components/                     (Header, UploadPanel, HeroCard, …)
│       └── utils/                          (format, export)
├── static/
│   ├── favicon.svg
│   └── manifest.webmanifest                (PWA manifest)
├── tests/unit/                             (Vitest suite — 38 tests)
├── svelte.config.js                        (adapter-static, paths.base)
├── vite.config.ts                          (Tailwind v4 plugin, pdfjs exclude)
├── eslint.config.js                        (flat config, ESLint 9)
└── .github/workflows/                      (CI + GitHub Pages deploy)
```

---

## Curriculum coverage

Curriculum data lives in `src/lib/data/{gen_ed,major}.json` and is consumed by the auditor verbatim. To update the curriculum, edit the JSON only — no code changes required.

| Category          | Required | Notes                                                                                                          |
| ----------------- | -------- | -------------------------------------------------------------------------------------------------------------- |
| General Education | 30 cr    | 6 strands with `choose_all`, `choose_one`, `choose_sequential_pair`, `choose_all_sub_groups` rules + electives |
| Basic Science     | 12 cr    | 6 mandatory courses                                                                                            |
| Core Courses      | 56 cr    | ~13 mandatory courses                                                                                          |
| Capstone          | 3–6 cr   | Choose 1 of ~5 options                                                                                         |
| Major Electives   | 12 cr    | Domains + clusters; complete at least N clusters                                                               |
| Free Electives    | 6 cr     | Auto-detected from unused passing courses                                                                      |

---

## Tech stack

| Layer           | Choice                                                                   |
| --------------- | ------------------------------------------------------------------------ |
| Framework       | SvelteKit 2 + Svelte 5 (Runes)                                           |
| Language        | TypeScript 5                                                             |
| Styling         | Tailwind CSS v4 (CSS-first config)                                       |
| PDF parsing     | `pdfjs-dist` (npm, dynamically imported)                                 |
| State           | Svelte 5 Runes (`$state`, `$derived`, `$effect`) in `.svelte.ts` modules |
| Persistence     | Opt-in `localStorage` adapter                                            |
| PWA             | Native SvelteKit Service Worker                                          |
| Tests           | Vitest                                                                   |
| Build           | Vite + `@sveltejs/adapter-static`                                        |
| Deployment      | GitHub Pages (subpath `/course-audit-system/`)                           |
| Package manager | pnpm                                                                     |

---

## License

MIT — see [`LICENSE`](LICENSE) if present, otherwise the repository default.
