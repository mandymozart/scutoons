// The unit chosen per kind of field, remembered in this browser (owner 2026-10-18; units.ts): `position` → `px`,
// `stroke` → `pt` … One choice per kind, so X and Y (or every stroke width) switch together.

const KEY = 'scutoons.units';

function read(): Record<string, string> {
	try {
		const stored = JSON.parse(localStorage.getItem(KEY) ?? '{}');
		return stored && typeof stored === 'object' ? stored : {};
	} catch {
		return {};
	}
}

let prefs = $state<Record<string, string>>(typeof localStorage === 'undefined' ? {} : read());

export const unitPrefs = {
	get(kind: string, fallback: string): string {
		return prefs[kind] ?? fallback;
	},
	set(kind: string, id: string) {
		prefs = { ...prefs, [kind]: id };
		try {
			localStorage.setItem(KEY, JSON.stringify(prefs));
		} catch {
			// not remembered – fine
		}
	}
};
