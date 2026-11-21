// A Library tile dropped on a tree row (a slice row, a mood cell): the row says what it takes, the core answers
// whether the file fits; a drop that doesn't fit says why.
import type { LibraryFitAnswer, SlotFit } from '@scutoons/media';
import { library } from '$lib/editor/library.svelte.ts';
import { toast } from '$lib/toasts.ts';
import { m } from '$lib/paraglide/messages.js';

export const ASSET_DRAG = 'application/x-scutoons-asset';

/** a Library tile is being dragged (its data can only be read on the drop) */
export const isAssetDrag = (e: DragEvent): boolean => !!e.dataTransfer?.types.includes(ASSET_DRAG);

/** the misfit in words */
export function misfitWords(answer: LibraryFitAnswer): string {
	if (answer.ok) return '';
	if ('unknown' in answer) return m['library.drop_unknown']();
	if (answer.square) return m['library.drop_not_square']();
	return m['library.drop_misfit']({ got: m[`library.kind_${answer.got}`](), wants: answer.wants.map((k) => m[`library.kind_${k}`]()).join(' / ') });
}

/** the drop of a tile on a row that takes `fit`: `apply` with the file when it fits, else the reason as a toast */
export function dropAsset(e: DragEvent, fit: SlotFit, apply: (path: string) => void): boolean {
	const raw = e.dataTransfer?.getData(ASSET_DRAG);
	if (!raw) return false;
	e.preventDefault();
	let path = '';
	try {
		path = String(JSON.parse(raw).path ?? '');
	} catch {
		return false;
	}
	const answer = library.takes(path, fit);
	if (answer.ok) apply(path);
	else toast(misfitWords(answer), { type: 'warning' });
	return answer.ok;
}
