import { browser } from '$app/environment';
import type { AuditResult, ParseMode } from '$lib/audit/models';

const KEY_SAVE_ENABLED = 'audit_save_enabled';
const KEY_AUDIT = 'audit_v1';
const KEY_MODE = 'audit_mode_v1';
const KEY_THEME = 'theme';
const KEY_LANG = 'lang';

export const persistence = {
	readBool(key: string): boolean | null {
		if (!browser) return null;
		try {
			const v = localStorage.getItem(key);
			if (v === null) return null;
			return v === '1' || v === 'true';
		} catch {
			return null;
		}
	},
	writeBool(key: string, value: boolean): void {
		if (!browser) return;
		try {
			localStorage.setItem(key, value ? '1' : '0');
		} catch {
			/* ignore */
		}
	},
	readString(key: string): string | null {
		if (!browser) return null;
		try {
			return localStorage.getItem(key);
		} catch {
			return null;
		}
	},
	writeString(key: string, value: string): void {
		if (!browser) return;
		try {
			localStorage.setItem(key, value);
		} catch {
			/* ignore */
		}
	},
	remove(key: string): void {
		if (!browser) return;
		try {
			localStorage.removeItem(key);
		} catch {
			/* ignore */
		}
	},
	keys: {
		saveEnabled: KEY_SAVE_ENABLED,
		audit: KEY_AUDIT,
		mode: KEY_MODE,
		theme: KEY_THEME,
		lang: KEY_LANG
	}
};

export function loadSavedAudit(): AuditResult | null {
	const raw = persistence.readString(KEY_AUDIT);
	if (!raw) return null;
	try {
		return JSON.parse(raw) as AuditResult;
	} catch {
		return null;
	}
}

export function loadSavedMode(): ParseMode | null {
	const raw = persistence.readString(KEY_MODE);
	if (raw === 'grade' || raw === 'registration') return raw;
	return null;
}
