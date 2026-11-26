// Pure SIS page parsing. No browser, no network, no real student data.
// scrape.mjs feeds it a snapshot of main.innerText plus each section's table;
// parse.test.mjs feeds it synthetic fixtures.

const HIDDEN = '[ซ่อน]';

const DAY_NAMES = [
  ['พฤหัสบดี', 4],
  ['พฤหัส', 4],
  ['จันทร์', 1],
  ['อังคาร', 2],
  ['พุธ', 3],
  ['ศุกร์', 5],
  ['เสาร์', 6],
  ['อาทิตย์', 7],
  ['Thursday', 4],
  ['Tuesday', 2],
  ['Wednesday', 3],
  ['Saturday', 6],
  ['Sunday', 7],
  ['Monday', 1],
  ['Friday', 5],
  ['Thu', 4],
  ['Tue', 2],
  ['Wed', 3],
  ['Sat', 6],
  ['Sun', 7],
  ['Mon', 1],
  ['Fri', 5],
];

const DAY_RE = [...DAY_NAMES]
  .sort((a, b) => b[0].length - a[0].length)
  .map(([name]) => name)
  .join('|');

// "3 หน่วยกิต · 3(3-0-6)", "2 Credits • 2((2)-0-4)", or a bare "3 หน่วยกิต".
const CREDIT_RE = /^(\d+(?:\.\d+)?)\s*(?:หน่วยกิต|credits?)\s*(?:[·•∙・|]\s*|-\s+)?(.*)$/i;

const MEETING_ID = '\\d{3}[\\s-]\\d{4}[\\s-]\\d{4}|\\d{3}[\\s-]\\d{3}[\\s-]\\d{4}|\\d{3}[\\s-]\\d{3}[\\s-]\\d{3}|\\d{9,11}';
const MEETING_RE = () =>
  new RegExp(`((?:Zoom(?:\\s*Meeting)?|Meeting)\\s*ID|รหัส(?:การ)?ประชุม)(\\s*[:：]?\\s*)(${MEETING_ID})`, 'gi');

function dayNumber(name) {
  const hit = DAY_NAMES.find(([n]) => n.toLowerCase() === String(name).toLowerCase());
  return hit ? hit[1] : null;
}

function hm(value) {
  const [h, m] = String(value).replace('.', ':').split(':');
  return `${h.padStart(2, '0')}:${m}`;
}

/** "อังคาร 10:00–11:50" or "Tue 08.00—09.50" -> [{day, start, end}, …] */
export function parseTimes(text) {
  if (!text) return [];
  const re = new RegExp(
    `(${DAY_RE})\\s*(\\d{1,2}[.:]\\d{2})\\s*(?:น\\.?)?\\s*[\\u2013\\u2014-]\\s*(\\d{1,2}[.:]\\d{2})`,
    'gi'
  );
  return [...String(text).matchAll(re)]
    .map(([, day, start, end]) => ({ day: dayNumber(day), start: hm(start), end: hm(end) }))
    .filter((slot) => slot.day != null);
}

/**
 * Status line from the search page.
 * "แสดง 1–10 จาก 38" and "Showing 1-10 of 38" both count; null means no range
 * (no results, or a layout we cannot paginate).
 */
export function parsePageRange(status) {
  const m = String(status || '').match(/(\d+)\s*(?:[\u2013\u2014-]|to|ถึง)\s*(\d+)\s*(?:จาก|of)\s*(\d+)/i);
  if (!m) return null;
  return { start: Number(m[1]), end: Number(m[2]), total: Number(m[3]) };
}

export function isLastPage(status) {
  const range = parsePageRange(status);
  return !range || range.end >= range.total;
}

/** Course code from a search-result link, wherever it sits in the text. */
export function courseCodeFromText(text) {
  const m = String(text || '').match(/[0-9A-Z]{3}-\d{3}/i);
  return m ? m[0].toUpperCase() : '';
}

export function offeringId(href) {
  const m = String(href || '').match(/subjects\/([^/?#]+)/);
  return m ? m[1] : '';
}

function splitSecret(text, start) {
  let i = start;
  let sawToken = false;
  while (i < text.length) {
    const token = text.slice(i).match(/^\S+/)?.[0];
    if (!token) break;
    const stopLabel = /^(?:\*\*|Zoom\b|Meeting\b|Passcode\b|Password\b|รหัส)/i.test(token);
    const thai = /[ก-๙]/.test(token);
    if (sawToken && (token === HIDDEN || stopLabel || thai)) break;
    if (!sawToken && (token.startsWith('**') || thai)) break;
    sawToken = true;
    i += token.length;
    const ws = text.slice(i).match(/^\s+/)?.[0] ?? '';
    const next = text.slice(i + ws.length).match(/^\S+/)?.[0] ?? '';
    const nextStops =
      !next ||
      next === HIDDEN ||
      next.startsWith('**') ||
      /[ก-๙]/.test(next) ||
      /^(?:Zoom\b|Meeting\b|Passcode\b|Password\b|รหัส)/i.test(next);
    if (nextStops) break;
    i += ws.length;
  }
  return i;
}

function redactPasscodes(text) {
  const re = /(Passcode|Password|รหัสผ่าน)\s*[:：]?\s*/gi;
  let out = '';
  let cursor = 0;
  for (const m of text.matchAll(re)) {
    const valueStart = m.index + m[0].length;
    if (valueStart < cursor) continue;
    const secretEnd = splitSecret(text, valueStart);
    out += text.slice(cursor, m.index);
    if (secretEnd === valueStart) {
      out += m[0];
      cursor = valueStart;
      continue;
    }
    out += `${m[1]} ${HIDDEN}`;
    cursor = secretEnd;
  }
  return out + text.slice(cursor);
}

/**
 * Strip student IDs, Zoom/meeting IDs and passcodes from a "ผู้เรียน" note.
 * Already-redacted notes stay unchanged.
 */
export function redactNote(text) {
  if (text == null) return null;
  const s = String(text)
    .replace(/\b\d{10}\b/g, HIDDEN)
    .replace(MEETING_RE(), `$1$2${HIDDEN}`)
    .replace(/https?:\/\/(?:[\w.-]+\.)?zoom\.us\/\S+/gi, HIDDEN);
  return redactPasscodes(s);
}

function fieldAfter(lines, label) {
  const same = new RegExp(`^${label}\\s*[:：]\\s*(.+)$`, 'i');
  for (let i = 0; i < lines.length; i++) {
    const m = lines[i].match(same);
    if (m) return m[1].trim();
    if (lines[i].toLowerCase() === label.toLowerCase()) return lines[i + 1] ?? null;
  }
  return null;
}

function headingBoundary(line) {
  // Thai has no \b, so labels are bounded by space, a colon, or end of line.
  // "คณะวิทยาศาสตร์" is a value, not the "คณะ" label.
  return (
    /^(?:คณะ|Faculty)(?:\s*[:：]|\s|$)/i.test(line) ||
    /^(?:ภาควิชา|Department)(?:\s*[:：]|\s|$)/i.test(line) ||
    /^(?:เลือก\s*)?section\b/i.test(line) ||
    /^เลือก(?:\s|$)/.test(line) ||
    /^(?:กลุ่มที่|กลุ่ม|ตอน)(?:\s|[:：]|$)/.test(line)
  );
}

function courseCodeNear(lines, creditIndex) {
  if (creditIndex >= 0) {
    for (let i = creditIndex - 1; i >= Math.max(0, creditIndex - 4); i--) {
      const m = lines[i].match(/^[0-9A-Z]{3}-\d{3}\b/i);
      if (m) return m[0].toUpperCase();
    }
    return null;
  }
  const top = lines.slice(0, 8).find((l) => /^[0-9A-Z]{3}-\d{3}\b/i.test(l));
  return top ? top.match(/^[0-9A-Z]{3}-\d{3}/i)[0].toUpperCase() : null;
}

function declaredSections(lines) {
  for (const line of lines) {
    const m = line.match(/(\d+)\s*sections?\b/i);
    if (m && /(เลือก|select|choose)/i.test(line)) return Number(m[1]);
  }
  return null;
}

function sectionLabel(lines) {
  const re = /^(?:Section|Sec\.?|กลุ่มที่|กลุ่ม|ตอน)\s*[:：]?\s*(.+)$/i;
  const line = lines.find((l) => re.test(l));
  return line ? line.match(re)[1].trim() : '';
}

/** First number that belongs to a seat line. 0 is a real count (closed section). */
function parseSeats(lines) {
  for (const line of lines) {
    if (/^(?:ผู้เรียน|Students)(?:\s*[:：]|\s|$)/i.test(line)) continue;
    const m = line.match(/ที่นั่ง(?:ว่าง|คงเหลือ)?\s*[:：]?\s*(\d+)/) || line.match(/(\d+)\s*ที่นั่ง/);
    if (m) return Number(m[1]);
  }
  return null;
}

function tagged(lines, labels) {
  const re = new RegExp(`^(?:${labels.join('|')})\\s*[:：]\\s*(.*)$`, 'i');
  const line = lines.find((l) => re.test(l));
  if (!line) return null;
  const value = line.match(re)[1].trim();
  return value || null;
}

function studentsOf(lines) {
  const re = /^(?:ผู้เรียน|Students)\s*[:：]\s*(.*)$/i;
  const line = lines.find((l) => re.test(l));
  if (!line) return null;
  const value = line.match(re)[1].trim();
  return value ? redactNote(value) : null;
}

function parseSection(article) {
  const lines = (article?.lines || []).map((l) => String(l).trim()).filter(Boolean);
  const rows = (article?.rows || []).filter((r) => Array.isArray(r) && r.some((cell) => String(cell || '').trim()));
  return {
    section: sectionLabel(lines),
    seats: parseSeats(lines),
    students: studentsOf(lines),
    rows: rows.map((r) => ({ times: r[0] || '', rooms: r[1] || '', teachers: r[2] || '' })),
    midterm: tagged(lines, ['สอบกลางภาค', 'Midterm']),
    final: tagged(lines, ['สอบปลายภาค', 'Final']),
  };
}

/** Snapshot -> the intermediate readDetail used to build inside the page. */
export function parseDetail(snapshot) {
  const lines = (snapshot?.lines || []).map((l) => String(l).trim()).filter(Boolean);
  const creditIndex = lines.findIndex((l) => CREDIT_RE.test(l));
  const code = courseCodeNear(lines, creditIndex);
  const codeIndex = code == null ? -1 : lines.findIndex((l) => l.toUpperCase().startsWith(code));
  const anchor = creditIndex >= 0 ? creditIndex : codeIndex;
  const after = anchor >= 0 ? lines.slice(anchor + 1) : [];
  const stop = after.findIndex(headingBoundary);
  const block = after.slice(0, stop === -1 ? 4 : stop);
  const fee = block.find((l) => /บาท$/.test(l)) ?? null;
  const names = block.filter((l) => l !== fee);
  return {
    code,
    credit_line: creditIndex >= 0 ? lines[creditIndex] : '',
    name_th: names[0] ?? null,
    name_en: names[1] ?? null,
    fee,
    faculty: fieldAfter(lines, 'คณะ') ?? fieldAfter(lines, 'Faculty'),
    department: fieldAfter(lines, 'ภาควิชา') ?? fieldAfter(lines, 'Department'),
    declared_sections: declaredSections(lines),
    sections: (snapshot?.articles || []).map(parseSection),
  };
}

function meetingsFromRows(rows) {
  let body = rows || [];
  // A header row ("เวลา / ห้อง / ผู้สอน") has no clock time. A table with no
  // header starts at the first meeting, so dropping row 0 would lose it.
  if (body.length && parseTimes(body[0].times).length === 0) body = body.slice(1);
  return body.flatMap((r) => {
    const times = parseTimes(r.times);
    const rooms = String(r.rooms || '').split(',').map((x) => x.trim());
    return times.map((t, i) => ({
      ...t,
      // "L4, L3" is one room per meeting when the counts match. A single
      // meeting in "CS201, BSc0605(LAB)" keeps the whole cell.
      room: rooms.length === times.length ? rooms[i] : r.rooms || '',
      teachers: r.teachers || '',
    }));
  });
}

export function normalize(raw = {}, href = '') {
  const credit = String(raw?.credit_line || '');
  const m = credit.match(CREDIT_RE);
  const sections = Array.isArray(raw?.sections) ? raw.sections : [];
  return {
    id: offeringId(href),
    code: raw?.code ?? null,
    name_th: raw?.name_th ?? null,
    name_en: raw?.name_en ?? null,
    credits: m ? Number(m[1]) : null,
    credit_pattern: m && m[2].trim() ? m[2].trim() : null,
    faculty: raw?.faculty ?? null,
    department: raw?.department ?? null,
    fee: raw?.fee ?? null,
    sections: sections.map((s) => ({
      section: s?.section || '',
      seats: s?.seats ?? null,
      students: s?.students ?? null,
      meetings: meetingsFromRows(s?.rows),
      midterm: s?.midterm ?? null,
      final: s?.final ?? null,
    })),
    incomplete: raw?.declared_sections != null && raw.declared_sections > sections.length,
  };
}
