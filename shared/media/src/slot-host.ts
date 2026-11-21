// How a media slot gets a file: it asks the app's host (setupMedia's `host`) for a pick and hands a drag over it to the
// host; the creator answers with its Library. The fit is the host's to decide (the creator's: the core's `takes`).
import type { LibraryKind, SlotFit } from './library/types.ts';

/** what a slot holds */
export type SlotKind = 'image' | 'video' | 'audio' | 'model' | 'archive' | 'document';

/** one file of the picked file's family (its levels, its still) */
export interface PickedFile {
	path: string;
	kind: LibraryKind;
	quality: string | null;
}

/** a file chosen for a slot: its CDN-root path and its family's files */
export type SlotApply = (path: string, files: PickedFile[]) => void;

export interface SlotPick extends SlotFit {
	/** the slot's name, shown while picking */
	label: string;
	/** the slot's file now (CDN-root path), '' when empty */
	value: string;
	/** where a new file for this slot belongs (a CDN-root folder, e.g. catpope/s1/) */
	place: string;
	onpick: SlotApply;
}

/** a host's answer while a drag is over a slot */
export type DropAnswer = { ok: true } | { ok: false; why: string };

export interface MediaHost {
	pick(request: SlotPick): void;
	/** a drag over a slot: null when it isn't a file the host hands out */
	over(event: DragEvent, fit: SlotFit): DropAnswer | null;
	/** the drop: `apply` when it fits (true), else the host says why */
	drop(event: DragEvent, fit: SlotFit, apply: SlotApply): boolean;
}

/** what a slot of `kind` takes, as the Library names it */
export function slotFit(kind: SlotKind): SlotFit {
	return { kinds: [kind === 'archive' || kind === 'document' ? 'file' : kind] };
}
