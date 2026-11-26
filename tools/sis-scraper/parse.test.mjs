import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  courseCodeFromText,
  isLastPage,
  normalize,
  parseDetail,
  parsePageRange,
  parseTimes,
  redactNote,
} from './parse.mjs';

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const fixtures = join(dirname(fileURLToPath(import.meta.url)), 'fixtures');

test('synthetic detail fixtures', () => {
  const files = readdirSync(fixtures).filter((name) => name.endsWith('.json')).sort();
  assert.ok(files.length >= 10, `expected a fixture per layout, found ${files.length}`);
  for (const name of files) {
    const fx = JSON.parse(readFileSync(join(fixtures, name), 'utf8'));
    assert.deepEqual(normalize(parseDetail(fx), fx.href), fx.expect, name);
  }
});

test('empty and partial snapshots do not throw', () => {
  const course = normalize(parseDetail(null), '/explore/subjects/1');
  assert.equal(course.code, null);
  assert.equal(course.credits, null);
  assert.equal(course.faculty, null);
  assert.deepEqual(course.sections, []);
  assert.equal(course.incomplete, false);
  assert.deepEqual(normalize(undefined, ''), {
    id: '',
    code: null,
    name_th: null,
    name_en: null,
    credits: null,
    credit_pattern: null,
    faculty: null,
    department: null,
    fee: null,
    sections: [],
    incomplete: false,
  });
});

test('search status paginates in Thai and English', () => {
  assert.deepEqual(parsePageRange('แสดง 1–10 จาก 38 รายวิชา'), { start: 1, end: 10, total: 38 });
  assert.equal(isLastPage('แสดง 1-10 จาก 38'), false);
  assert.equal(isLastPage('แสดง 31–38 จาก 38'), true);
  assert.equal(isLastPage('Showing 1-10 of 38 courses'), false);
  assert.equal(isLastPage('Showing 31—38 of 38'), true);
  assert.equal(isLastPage('Showing 1 to 10 of 38'), false);
  assert.equal(isLastPage('แสดง 1 ถึง 10 จาก 20'), false);
  assert.equal(parsePageRange('ไม่พบรายวิชา'), null);
  assert.equal(isLastPage('ไม่พบรายวิชา'), true);
  assert.equal(isLastPage('No result'), true);
  assert.equal(isLastPage(''), true);
});

test('course code is found inside link text', () => {
  assert.equal(courseCodeFromText('344-101'), '344-101');
  assert.equal(courseCodeFromText('  344-101\nชื่อวิชา'), '344-101');
  assert.equal(courseCodeFromText('รหัส 344-101'), '344-101');
  assert.equal(courseCodeFromText('ชื่อวิชาก่อน 322-101 CALCULUS'), '322-101');
  assert.equal(courseCodeFromText('ไม่มีรหัส'), '');
});

test('clock times cover dotted hours, English days and em dashes', () => {
  assert.deepEqual(parseTimes('จันทร์ 9:00-10:05'), [{ day: 1, start: '09:00', end: '10:05' }]);
  assert.deepEqual(parseTimes('Sunday 13:00—14:00'), [{ day: 7, start: '13:00', end: '14:00' }]);
  assert.deepEqual(parseTimes(''), []);
  assert.deepEqual(parseTimes(null), []);
});

test('notes redact ids and passcodes without eating the rest of the line', () => {
  assert.equal(
    redactNote('ทั่วไป 6700000001 Meeting ID: 111 2222 3333'),
    'ทั่วไป [ซ่อน] Meeting ID: [ซ่อน]'
  );
  assert.equal(redactNote('Zoom Meeting ID: 11122223333'), 'Zoom Meeting ID: [ซ่อน]');
  assert.equal(redactNote('รหัสประชุม: 111-222-3333'), 'รหัสประชุม: [ซ่อน]');
  assert.equal(redactNote('รหัสผ่าน: 9999 แล้วพบกัน'), 'รหัสผ่าน [ซ่อน] แล้วพบกัน');
  assert.equal(redactNote('พ.ศ.2568 และ พ.ศ. 2569'), 'พ.ศ.2568 และ พ.ศ. 2569');
  const already = 'ทั่วไป (Zoom ID: [ซ่อน] Passcode [ซ่อน] **นัดคาบแรก 25 มิ.ย. 69 เวลา 17.00 น.**';
  assert.equal(redactNote(already), already);
  assert.equal(redactNote('ZOOM ID [ซ่อน] Passcode [ซ่อน] นัดอธิบาย'), 'ZOOM ID [ซ่อน] Passcode [ซ่อน] นัดอธิบาย');
  assert.equal(redactNote('ZoomID : [ซ่อน] Passcode [ซ่อน] นัด ONSITE'), 'ZoomID : [ซ่อน] Passcode [ซ่อน] นัด ONSITE');
  assert.equal(redactNote(null), null);
});

test('CI secret guard matches the leaks redaction removes', () => {
  const pattern = secretGuard('.github/workflows/ci.yml');
  assert.equal(secretGuard('.github/workflows/offerings.yml').source, pattern.source);
  const leaked = [
    '6700000001',
    'Passcode: abc',
    'Passcode 1234',
    'Password: s3cret',
    'รหัสผ่าน: 9999',
    'Zoom ID: 11122223333',
    'Meeting ID: 111 2222 3333',
    'https://us02web.zoom.us/j/11122223333?pwd=notreal',
  ];
  for (const line of leaked) assert.match(line, pattern, line);
  for (const line of leaked) assert.doesNotMatch(redactNote(line), pattern, redactNote(line));
  assert.doesNotMatch('CS201, BSc0605(LAB)', pattern);
  assert.doesNotMatch('พ.ศ.2568', pattern);
  assert.doesNotMatch('Passcode [ซ่อน]', pattern);
  assert.doesNotMatch('Meeting ID: [ซ่อน]', pattern);
});

function secretGuard(rel) {
  const yml = readFileSync(join(root, rel), 'utf8');
  const m = yml.match(/grep -niE '([^']+)' data\/offerings/);
  assert.ok(m, `${rel} is missing the offerings secret guard`);
  return new RegExp(m[1], 'i');
}
