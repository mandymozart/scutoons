/**
 * Until 2026-10-06 the creator's true/false fields (PropField type "bool") stored the strings "true" / "false"; the
 * schemas want booleans, so a save of such a document was refused (e.g. a VN event's `wait` – "must match the const
 * value"). On load, these fields' strings become booleans again (the document is then unsaved: the next save stores
 * the repair). Only the keys of those fields, only the exact strings "true" / "false".
 */
export const BOOLEAN_KEYS = new Set([
	// group renderer params (swiper)
	'enabled',
	'loop',
	'flip',
	'cube',
	'keyboard',
	'freeMode',
	// VN events: params.wait (trigger.once / a slice item's once are gone: `repeat`, migration 2026-10-06-repeat)
	'wait',
	// slice audio samples: loop, trigger.exclusive, trigger.gate
	'exclusive',
	'gate',
	// text boxes
	'uppercase',
	'frame'
]);

/** repairs `value` in place; the number of fields changed */
export function repairBooleans(value: unknown): number {
	if (Array.isArray(value)) return value.reduce((n: number, item) => n + repairBooleans(item), 0);
	if (value === null || typeof value !== 'object') return 0;
	let changed = 0;
	const record = value as Record<string, unknown>;
	for (const [key, field] of Object.entries(record)) {
		if (BOOLEAN_KEYS.has(key) && (field === 'true' || field === 'false')) {
			record[key] = field === 'true';
			changed++;
		} else changed += repairBooleans(field);
	}
	return changed;
}
