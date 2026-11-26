// Scrapes public course offerings from PSU SIS (https://sis.psu.ac.th/explore/subject)
// into data/offerings/<year>-<term>.json. SIS is a Blazor Server app (data arrives
// over SignalR, nothing is server-rendered), so a real browser drives the pages.
//
// Usage:
//   node scrape.mjs                      # every term SIS lists, curriculum courses
//   node scrape.mjs --latest 2 --ge      # the 2 newest terms, plus GenEd prefixes (weekly CI job)
//   node scrape.mjs --terms 1/2569,2/2569 --ge
//   node scrape.mjs --keywords 344-,322-101 --terms 2/2569
//
// A run with the default keywords replaces each term's file, so withdrawn courses
// disappear; --keywords merges into it. A file is only rewritten when its courses
// change, so scheduled runs do not create empty commits.
// Only public pages are visited (no login). Requests are sequential with a delay.

import { chromium } from 'playwright-core';
import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { courseCodeFromText, isLastPage, normalize, parseDetail } from './parse.mjs';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const OUT = join(ROOT, 'data', 'offerings');
const BASE = 'https://sis.psu.ac.th';
const CAMPUS = '01'; // Hat Yai
const DELAY_MS = 700;

const args = Object.fromEntries(
  process.argv.slice(2).reduce((acc, a, i, all) => {
    if (a.startsWith('--')) acc.push([a.slice(2), all[i + 1]?.startsWith('--') || all[i + 1] === undefined ? true : all[i + 1]]);
    return acc;
  }, [])
);

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** Course codes the curricula reference outside the 344- prefix, plus GE prefixes. */
function curriculumKeywords() {
  const codes = new Set();
  const gePrefixes = new Set();
  for (const id of ['cs2564', 'cs2569']) {
    const c = JSON.parse(readFileSync(join(ROOT, 'crates/audit-core/src/data', `${id}.json`), 'utf8'));
    for (const x of c.major.basic_science.courses) codes.add(x.code);
    for (const x of c.major.core_courses.courses) codes.add(x.code);
    const ge = JSON.stringify(c.gen_ed);
    for (const m of ge.matchAll(/"code":\s*"([0-9A-Z]{3})-\d{3}"/g)) gePrefixes.add(`${m[1]}-`);
    for (const p of c.gen_ed.electives.language_prefixes || []) gePrefixes.add(p);
  }
  const major = ['344-', ...[...codes].filter((c) => !c.startsWith('344-')).sort()];
  return { major, ge: [...gePrefixes].sort() };
}

async function waitForStatus(page) {
  // "แสดง 1–10 จาก 38 รายวิชา …" or a no-result message.
  await page.waitForFunction(
    () => {
      const s = document.querySelector('[role=status]')?.textContent || '';
      return /แสดง|ไม่พบ|No result|Showing/.test(s);
    },
    null,
    { timeout: 30000 }
  );
  return page.$eval('[role=status]', (e) => e.textContent.trim()).catch(() => '');
}

/** All result rows for one keyword in one term: [{code, href}] across pages. */
async function searchRows(page, term, year, keyword) {
  const url = `${BASE}/explore/subject?term=${term}&year=${year}&keyword=${encodeURIComponent(keyword)}&c=${CAMPUS}`;
  await page.goto(url, { waitUntil: 'domcontentloaded' });
  let status = await waitForStatus(page);
  const rows = new Map();
  for (let guard = 0; guard < 200; guard++) {
    const pageRows = await page.$$eval('a[href^="/explore/subjects/"]', (as) =>
      as.map((a) => ({ href: a.getAttribute('href').split('?')[0], text: a.textContent || '' }))
    );
    for (const r of pageRows) {
      const code = courseCodeFromText(r.text);
      if (code) rows.set(r.href, { href: r.href, code });
    }
    if (isLastPage(status)) break;
    const before = status;
    await page.click('.sis-pagination [aria-label="หน้าถัดไป"]');
    await page.waitForFunction(
      (prev) => (document.querySelector('[role=status]')?.textContent || '').trim() !== prev,
      before,
      { timeout: 30000 }
    );
    status = await waitForStatus(page);
    await sleep(DELAY_MS);
  }
  return [...rows.values()];
}

/** Parse one course detail page. */
async function readDetail(page, href) {
  await page.goto(BASE + href, { waitUntil: 'domcontentloaded' });
  await page.waitForFunction(
    () => {
      const m = document.querySelector('main');
      return (
        m &&
        /หน่วยกิต|credits/i.test(m.innerText) &&
        (m.querySelector('article') || /ไม่มี|ไม่พบ|no section|not found|no result/i.test(m.innerText))
      );
    },
    null,
    { timeout: 30000 }
  );
  await sleep(300); // let remaining sections render
  const snapshot = await page.evaluate(() => {
    const main = document.querySelector('main');
    const lines = main.innerText.split('\n').map((l) => l.trim()).filter(Boolean);
    const articles = [...main.querySelectorAll('article')].map((a) => ({
      lines: a.innerText.split('\n').map((l) => l.trim()).filter(Boolean),
      rows: [...a.querySelectorAll('tr')]
        .map((tr) => [...tr.querySelectorAll('td')].map((td) => td.innerText.trim()))
        .filter((r) => r.length),
    }));
    return { lines, articles };
  });
  return parseDetail(snapshot);
}

async function availableTerms(page) {
  const res = await page.goto(`${BASE}/explore/subject`, { waitUntil: 'domcontentloaded' });
  try {
    await page.waitForSelector('button.sis-select-field-box', { timeout: 30000 });
  } catch (e) {
    // Say what SIS served instead (blocked IP, maintenance page, ...) before failing.
    const body = await page.evaluate(() => document.body?.innerText ?? '').catch(() => '');
    console.error(`SIS term picker missing: HTTP ${res?.status()} ${page.url()} title=${JSON.stringify(await page.title())}`);
    console.error(body.slice(0, 500));
    throw e;
  }
  await sleep(1500); // let the Blazor circuit attach handlers
  await page.click('button.sis-select-field-box');
  await page.waitForSelector('[role=option].sis-select-field-option', { timeout: 15000 });
  const terms = await page.$$eval('[role=option].sis-select-field-option', (els) =>
    els.map((e) => e.textContent.trim()).filter((t) => /^\d\/25\d\d$/.test(t))
  );
  await page.keyboard.press('Escape');
  return terms;
}

async function main() {
  const browser = await chromium.launch({ channel: 'chrome', headless: !args.headed });
  const page = await browser.newPage({ locale: 'th-TH' });
  const listed = args.terms ? String(args.terms).split(',') : await availableTerms(page);
  const terms = args.latest ? listed.slice(0, Number(args.latest)) : listed;
  const kw = curriculumKeywords();
  const keywords = args.keywords ? String(args.keywords).split(',') : [...kw.major, ...(args.ge ? kw.ge : [])];
  mkdirSync(OUT, { recursive: true });
  console.log(`terms: ${terms.join(' ')}\nkeywords (${keywords.length}): ${keywords.join(' ')}`);

  for (const t of terms) {
    const [term, year] = t.split('/');
    const file = join(OUT, `${year}-${term}.json`);
    const existing = existsSync(file) ? JSON.parse(readFileSync(file, 'utf8')) : { courses: [] };
    const fresh = new Map();
    let failures = 0;
    const started = Date.now();
    const hrefs = new Map();
    for (const k of keywords) {
      let rows = [];
      try {
        rows = await searchRows(page, term, year, k);
      } catch (e) {
        failures++;
        console.warn(`  ! search ${t} ${k}: ${e.message.split('\n')[0]}`);
      }
      // A code keyword like "322-101" also matches other text; keep exact-code hits.
      for (const r of rows) if (k.endsWith('-') ? r.code.startsWith(k) : r.code === k) hrefs.set(r.href, r);
      await sleep(DELAY_MS);
    }
    let n = 0;
    for (const { href } of hrefs.values()) {
      try {
        const course = normalize(await readDetail(page, href), href);
        if (course.code) fresh.set(course.id, course);
      } catch (e) {
        failures++;
        console.warn(`  ! ${href}: ${e.message.split('\n')[0]}`);
      }
      if (++n % 20 === 0) console.log(`  ${t}: ${n}/${hrefs.size}`);
      await sleep(DELAY_MS);
    }
    // Replace on a full default run without errors; otherwise merge so nothing is lost.
    const replace = !args.keywords && failures === 0;
    const byKey = new Map(replace ? [] : existing.courses.map((c) => [c.id, c]));
    for (const [id, c] of fresh) byKey.set(id, c);
    const courses = [...byKey.values()].sort((a, b) => a.code.localeCompare(b.code) || (a.name_en || '').localeCompare(b.name_en || ''));
    if (JSON.stringify(courses) === JSON.stringify(existing.courses)) {
      console.log(`${t}: unchanged (${courses.length} courses)`);
      continue;
    }
    const out = {
      term: `${term}/${year}`,
      campus: 'หาดใหญ่',
      source: `${BASE}/explore/subject`,
      scraped_at: new Date().toISOString().slice(0, 10),
      courses,
    };
    writeFileSync(file, JSON.stringify(out, null, 1) + '\n');
    console.log(`${t}: ${courses.length} courses (${hrefs.size} visited) in ${Math.round((Date.now() - started) / 1000)}s -> ${file}`);
  }

  // index of available term files, newest first
  const index = terms
    .map((t) => t.split('/'))
    .map(([term, year]) => `${year}-${term}`)
    .filter((k) => existsSync(join(OUT, `${k}.json`)));
  const prev = existsSync(join(OUT, 'index.json')) ? JSON.parse(readFileSync(join(OUT, 'index.json'), 'utf8')).terms : [];
  const all = [...new Set([...index, ...prev])].sort((a, b) => b.localeCompare(a));
  const indexJson = JSON.stringify({ terms: all }, null, 1) + '\n';
  if (indexJson !== JSON.stringify({ terms: prev }, null, 1) + '\n') writeFileSync(join(OUT, 'index.json'), indexJson);
  await browser.close();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
