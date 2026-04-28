import { settingsStore } from '$lib/stores/settingsStore.svelte';

type Messages = {
	[key: string]: { en: string; th: string };
};

const messages: Messages = {
	'app.title': { en: 'Course Audit', th: 'ตรวจสอบหลักสูตร' },
	'app.subtitle': {
		en: 'Privacy-first PSU Computer Science transcript auditor',
		th: 'เครื่องมือตรวจสอบใบเกรด CS ที่ปกป้องความเป็นส่วนตัว'
	},
	'app.privacyBadge': {
		en: '100% client-side',
		th: 'ประมวลผลในเครื่องคุณ 100%'
	},
	'header.toggleLang': {
		en: 'เปลี่ยนเป็นภาษาไทย',
		th: 'Switch to English'
	},
	'header.toggleTheme.dark': { en: 'Switch to dark mode', th: 'เปลี่ยนเป็นโหมดมืด' },
	'header.toggleTheme.light': { en: 'Switch to light mode', th: 'เปลี่ยนเป็นโหมดสว่าง' },
	'upload.drop': { en: 'Drop your PDF transcript here', th: 'วาง PDF ใบเกรดที่นี่' },
	'upload.or': { en: 'or', th: 'หรือ' },
	'upload.choose': { en: 'Choose a file', th: 'เลือกไฟล์' },
	'upload.hint': {
		en: 'PDFs only. Stays in your browser.',
		th: 'รับเฉพาะไฟล์ PDF เก็บอยู่ในเครื่องคุณ'
	},
	'upload.selected': { en: 'Selected', th: 'เลือกไว้' },
	'upload.analyze': { en: 'Analyze transcript', th: 'วิเคราะห์ใบเกรด' },
	'upload.analyzing': { en: 'Analyzing…', th: 'กำลังวิเคราะห์…' },
	'sidebar.save.label': { en: 'Save results in browser', th: 'บันทึกผลในเบราว์เซอร์' },
	'sidebar.save.hint': {
		en: 'Opt-in. Stored only on this device.',
		th: 'เปิดใช้เอง เก็บในเครื่องนี้เท่านั้น'
	},
	'results.empty.title': { en: 'No transcript loaded', th: 'ยังไม่มีใบเกรด' },
	'results.empty.body': {
		en: 'Upload a PDF to see your audit.',
		th: 'อัปโหลด PDF เพื่อดูการตรวจสอบ'
	},
	'results.totalCredits': { en: 'Total credits earned', th: 'หน่วยกิตรวมที่ได้' },
	'results.gpa': { en: 'GPA', th: 'GPA' },
	'results.registration.banner': {
		en: 'Registration mode: in-progress courses fill slots but do not count toward credits yet.',
		th: 'โหมดลงทะเบียน: วิชาที่ยังเรียนอยู่จะอยู่ในช่อง แต่ยังไม่นับหน่วยกิต'
	},
	'results.export.json': { en: 'Export JSON', th: 'ส่งออก JSON' },
	'results.export.print': { en: 'Print / Save PDF', th: 'พิมพ์ / บันทึก PDF' },
	'results.copyMissing': { en: 'Copy missing list', th: 'คัดลอกรายการที่ยังขาด' },
	'results.copied': { en: 'Copied!', th: 'คัดลอกแล้ว!' },
	'results.required': { en: 'Required', th: 'ที่ต้องการ' },
	'results.collected': { en: 'Earned', th: 'ที่ได้' },
	'category.GenEd': { en: 'General Education', th: 'หมวดวิชาศึกษาทั่วไป' },
	'category.Major': { en: 'Major Courses', th: 'หมวดวิชาเอก' },
	'category.FreeElectives': { en: 'Free Electives', th: 'หมวดวิชาเลือกเสรี' },
	'whatif.title': { en: 'What-if simulator', th: 'จำลองวิชาที่จะลงเรียน' },
	'whatif.body': {
		en: 'Add hypothetical courses to see your projected audit.',
		th: 'เพิ่มวิชาที่อยากลงเพื่อดูผลโดยประมาณ'
	},
	'whatif.code': { en: 'Course code', th: 'รหัสวิชา' },
	'whatif.name': { en: 'Course name', th: 'ชื่อวิชา' },
	'whatif.credit': { en: 'Credits', th: 'หน่วยกิต' },
	'whatif.grade': { en: 'Grade', th: 'เกรด' },
	'whatif.add': { en: 'Add', th: 'เพิ่ม' },
	'whatif.clear': { en: 'Clear all', th: 'ล้างทั้งหมด' },
	'whatif.empty': {
		en: 'No planned courses yet.',
		th: 'ยังไม่มีวิชาที่วางแผนไว้'
	},
	'whatif.remove': { en: 'Remove', th: 'ลบ' },
	'semester.title': { en: 'Semester breakdown', th: 'แยกตามภาคการศึกษา' },
	'semester.gpa': { en: 'Semester GPA', th: 'GPA ภาคการศึกษา' },
	'semester.credits': { en: 'credits', th: 'หน่วยกิต' },
	'semester.unspecified': { en: 'Unspecified semester', th: 'ไม่ระบุภาคการศึกษา' },
	'missing.title': { en: 'Missing requirements', th: 'รายการที่ยังขาด' },
	'missing.empty': {
		en: 'You have completed all requirements!',
		th: 'คุณเรียนครบทุกข้อกำหนดแล้ว!'
	},
	'reset.label': { en: 'Reset', th: 'ล้างข้อมูล' },
	'error.title': { en: 'Something went wrong', th: 'เกิดข้อผิดพลาด' }
};

export function t(key: string): string {
	const entry = messages[key];
	if (!entry) return key;
	return entry[settingsStore.lang];
}

export function tCategory(name: string): string {
	switch (name) {
		case 'General Education':
			return t('category.GenEd');
		case 'Major Courses':
			return t('category.Major');
		case 'Free Electives':
			return t('category.FreeElectives');
		default:
			return name;
	}
}
