// Reads YOUR co-op preparation hours (อบรมเตรียมความพร้อมสหกิจศึกษา 30 ชั่วโมง)
// from the Faculty of Science report at https://regis.sci.psu.ac.th/activity30hr.
// The report is a Looker Studio table per cohort; this filters it to one student ID
// and prints only that student's rows and totals. Nothing is saved.
//
// Usage: node coop-hours.mjs 67xxxxxxxx

import { chromium } from 'playwright-core';

// Embedded reports on regis.sci.psu.ac.th/activity30hr, by cohort (first 2 digits).
const REPORTS = {
  68: 'https://lookerstudio.google.com/embed/reporting/5fef4d4d-1390-4355-a8db-e474288de19d/page/GMqoF',
  67: 'https://lookerstudio.google.com/embed/reporting/1163cf47-a57e-4a7b-bc0c-3bf993066912/page/oMooF',
  66: 'https://lookerstudio.google.com/embed/reporting/3e4be6a9-e1e7-41c3-8a52-b322181fb4ff/page/e3UIE',
  65: 'https://lookerstudio.google.com/embed/reporting/4c2412cf-3357-4e3c-999c-c48f78df4a3c/page/C7YVD',
};
const REQUIRED = { faculty: 24, university: 6 };

const id = process.argv[2];
if (!/^\d{10}$/.test(id || '')) {
  console.error('usage: node coop-hours.mjs <10-digit student id>');
  process.exit(2);
}
const url = REPORTS[id.slice(0, 2)];
if (!url) {
  console.error(`no report for cohort ${id.slice(0, 2)} (known: ${Object.keys(REPORTS).join(', ')})`);
  process.exit(2);
}

const browser = await chromium.launch({ channel: 'chrome' });
try {
  const page = await browser.newPage({ locale: 'th-TH' });
  await page.goto(url, { waitUntil: 'domcontentloaded' });
  const box = page.getByRole('textbox', { name: 'ป้อนค่า' });
  await box.waitFor({ timeout: 60000 });
  await box.fill(id);
  await box.press('Enter');
  // Wait until the pager reports only this student's rows ("1 - N / N").
  await page.waitForFunction(
    (sid) => {
      const t = document.body.innerText;
      const m = t.match(/(\d+)\s*-\s*(\d+)\s*\/\s*(\d+)/);
      return m && m[2] === m[3] && t.includes(sid);
    },
    id,
    { timeout: 60000 }
  );
  const text = await page.evaluate(() => document.body.innerText);
  const lines = text.split('\n').map((l) => l.trim()).filter(Boolean);
  const rows = [];
  for (let i = 0; i < lines.length; i++) {
    if (lines[i] !== id) continue;
    const [activity, date, faculty, university] = lines.slice(i + 1, i + 5);
    const num = (v) => (/^\d+(\.\d+)?$/.test(v) ? Number(v) : 0);
    rows.push({ activity, date, faculty: num(faculty), university: num(university) });
  }
  const total = (k) => rows.reduce((s, r) => s + r[k], 0);
  const faculty = total('faculty');
  const university = total('university');
  console.log(
    JSON.stringify(
      {
        student: id,
        source: url,
        rows,
        faculty,
        university,
        required: REQUIRED,
        complete: faculty >= REQUIRED.faculty && university >= REQUIRED.university,
      },
      null,
      1
    )
  );
} finally {
  await browser.close();
}
