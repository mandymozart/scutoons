// Geometry is the same in every language (owner 2026-10-03: "There is no language-specific geometry"): where text boxes
// and placed items sit, their size, rotation and scale live in each locale file but must not differ – only the text is
// per language. `unifyGeometry` makes the other locales follow the source language (the creator runs it on load).

const DIALOG_GEOMETRY = ['rect', 'rotation', 'scale'] as const;
const ITEM_GEOMETRY = ['rect', 'rotation'] as const;

const same = (a: unknown, b: unknown) => JSON.stringify(a ?? null) === JSON.stringify(b ?? null);

function slicesOf(kind: string, file: any): any[] {
	if (!file || typeof file !== 'object') return [];
	if (kind === 'chapter') return (Array.isArray(file.groups) ? file.groups : []).flatMap((g: any) => (Array.isArray(g?.slices) ? g.slices : []));
	if (kind === 'group') return Array.isArray(file.slices) ? file.slices : [];
	return [];
}

function copyKeys(from: any, to: any, keys: readonly string[]): number {
	let changed = 0;
	for (const key of keys) {
		if (same(from?.[key], to?.[key])) continue;
		if (from?.[key] === undefined) delete to[key];
		else to[key] = JSON.parse(JSON.stringify(from[key]));
		changed++;
	}
	return changed;
}

/**
 * Copies the source locale's geometry (text boxes: rect, rotation, scale; placed items: rect, rotation) into every other
 * locale of a chapter / scene file, matched by position (text boxes and items are in the same order in every locale).
 * Mutates `files`; returns how many boxes / items were changed (0 = all the same already).
 */
export function unifyGeometry(kind: string, files: Record<string, any>, primary: string): number {
	const source = slicesOf(kind, files[primary]);
	let changed = 0;
	for (const [locale, file] of Object.entries(files)) {
		if (locale === primary || !file) continue;
		slicesOf(kind, file).forEach((slice, i) => {
			const from = source[i];
			if (!from || !slice) return;
			(Array.isArray(slice.dialog) ? slice.dialog : []).forEach((d: any, j: number) => {
				const f = from.dialog?.[j];
				if (f && d && copyKeys(f, d, DIALOG_GEOMETRY)) changed++;
			});
			(Array.isArray(slice.items) ? slice.items : []).forEach((it: any, j: number) => {
				const f = from.items?.[j];
				if (f && it && f.id === it.id && copyKeys(f, it, ITEM_GEOMETRY)) changed++;
			});
		});
	}
	return changed;
}
