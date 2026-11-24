// The Library as the creator's media host: a slot's pick opens the Library on what fits, a tile dragged over a slot or
// a tree row (a slice row, a mood cell) asks the core whether the file fits; a drop that doesn't fit says why.
import type { AssetFamily, DropAnswer, LibraryFitAnswer, MediaHost, PickedFile, SlotFit } from '@scutoons/media';
import type { RegistryCharacter } from '@scutoons/content';
import { library } from '$lib/editor/library.svelte.ts';
import { imageRatio, libraryPiece, type PickedPiece } from '$lib/editor/collage-pick.ts';
import { asset } from '$lib/utils/cdn.ts';
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

/** a family's files as a slot reads them (its levels, its still) */
const filesOf = (family: AssetFamily | null): PickedFile[] => family?.versions.map((v) => ({ path: v.path, kind: v.kind, quality: v.quality })) ?? [];

/** while a tile is over a target that takes `fit`: whether it fits, and why not (null: no tile is dragged) */
export function overAsset(e: DragEvent, fit: SlotFit): DropAnswer | null {
	if (!isAssetDrag(e)) return null;
	const answer = library.dragging ? library.takes(library.dragging, fit) : null;
	return !answer || answer.ok ? { ok: true } : { ok: false, why: misfitWords(answer) };
}

/** the drop of a tile on a target that takes `fit`: `apply` with the file when it fits, else the reason as a toast */
export function dropAsset(e: DragEvent, fit: SlotFit, apply: (path: string, files: PickedFile[]) => void): boolean {
	const raw = e.dataTransfer?.getData(ASSET_DRAG);
	library.dragging = null;
	if (!raw) return false;
	e.preventDefault();
	let path = '';
	try {
		path = String(JSON.parse(raw).path ?? '');
	} catch {
		return false;
	}
	const answer = library.takes(path, fit);
	if (answer.ok) apply(path, filesOf(library.familyOf(path)));
	else toast(misfitWords(answer), { type: 'warning', kind: 'library' });
	return answer.ok;
}

/** what answers every media slot in the creator (setupMedia's `host`) */
export const libraryHost: MediaHost = {
	// TODO(#701): an upload while picking goes to the slot's folder (`request.place`), not the open item's
	pick(request) {
		void library.load();
		library.startPick({
			label: request.label,
			kinds: request.kinds,
			square: request.square,
			value: request.value || undefined,
			onpick: (path) => request.onpick(path, filesOf(library.familyOf(path)))
		});
	},
	over: overAsset,
	drop: dropAsset
};

/** a collage piece from the Library: any picture; one without sizes gets its ratio by decoding it */
export function pickPiece(label: string, characters: readonly RegistryCharacter[], onpick: (piece: PickedPiece) => void) {
	void library.load();
	library.startPick({
		label,
		kinds: ['image'],
		onpick: (path) => {
			const piece = libraryPiece(path, library.familyOf(path), characters);
			if (piece.ratio) return onpick(piece);
			void imageRatio(asset(piece.src)).then((ratio) => onpick({ ...piece, ratio: ratio ?? undefined }));
		}
	});
}
