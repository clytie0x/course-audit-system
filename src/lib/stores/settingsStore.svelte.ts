import { browser } from '$app/environment';
import { persistence } from './persistence';

type Lang = 'en' | 'th';

function createSettingsStore() {
	let isDark = $state(false);
	let lang = $state<Lang>('en');
	let saveEnabled = $state(false);

	if (browser) {
		isDark = document.documentElement.classList.contains('dark');
		const storedLang = persistence.readString(persistence.keys.lang);
		if (storedLang === 'th' || storedLang === 'en') lang = storedLang;
		const storedSave = persistence.readBool(persistence.keys.saveEnabled);
		if (storedSave !== null) saveEnabled = storedSave;
	}

	function setDark(value: boolean) {
		isDark = value;
		if (!browser) return;
		document.documentElement.classList.toggle('dark', value);
		persistence.writeString(persistence.keys.theme, value ? 'dark' : 'light');
	}

	function toggleDark() {
		setDark(!isDark);
	}

	function setLang(value: Lang) {
		lang = value;
		if (browser) persistence.writeString(persistence.keys.lang, value);
	}

	function toggleLang() {
		setLang(lang === 'en' ? 'th' : 'en');
	}

	function setSaveEnabled(value: boolean) {
		saveEnabled = value;
		if (browser) {
			persistence.writeBool(persistence.keys.saveEnabled, value);
			if (!value) {
				persistence.remove(persistence.keys.audit);
				persistence.remove(persistence.keys.mode);
			}
		}
	}

	return {
		get isDark() {
			return isDark;
		},
		get lang() {
			return lang;
		},
		get isThai() {
			return lang === 'th';
		},
		get saveEnabled() {
			return saveEnabled;
		},
		setDark,
		toggleDark,
		setLang,
		toggleLang,
		setSaveEnabled
	};
}

export const settingsStore = createSettingsStore();
