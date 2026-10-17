// The chosen text size unit, remembered per browser (owner 2026-10-17; the conversions: text-size-unit.ts).
import { TEXT_SIZE_UNITS, type TextSizeUnit } from '$lib/editor/text-size-unit.ts';
import { shared } from '$lib/utils/shared.ts';

const KEY = 'scutoons.creator.textSizeUnit';

/** the chosen unit, remembered per browser */
class TextSizeUnitPref {
	value = $state<TextSizeUnit>('rem');

	constructor() {
		try {
			const stored = typeof localStorage === 'undefined' ? null : localStorage.getItem(KEY);
			if (stored && (TEXT_SIZE_UNITS as readonly string[]).includes(stored)) this.value = stored as TextSizeUnit;
		} catch {
			// private mode / blocked storage: the default
		}
	}

	set(unit: TextSizeUnit) {
		this.value = unit;
		try {
			localStorage.setItem(KEY, unit);
		} catch {
			// not remembered – fine
		}
	}
}

export const textSizeUnit = shared('text-size-unit', () => new TextSizeUnitPref());
