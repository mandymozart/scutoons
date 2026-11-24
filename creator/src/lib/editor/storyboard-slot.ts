// The shot's asset slot (lap 2 C3c, AI-HOOKS.md §0): a picture or a video from the Library, an upload, or a generator
// added from the Library – the generator stands in until a take is assigned. What fills it: the member's pick, else the
// shot's approved generation (the ledger's). Pure: test/storyboard-slot.test.ts.
//
// TODO(#877): the storyboard document has no field for a shot's asset yet; until it has, a pick is kept per storyboard in
// this browser (the document and its saving stay as they are).
import { kindOf } from '@scutoons/content/storyboard';

/** what a member put into a shot's slot: a Library file (its path) or a generator (its address, `generator:<id>`) */
export type SlotPick = { file: string } | { generator: string; name?: string };

/** what the slot shows */
export type SlotContent =
	| { at: 'file'; path: string; video: boolean }
	| { at: 'generator'; address: string; name: string }
	| { at: 'approved'; generation: number; video: boolean }
	| { at: 'empty' };

interface Store {
	getItem(key: string): string | null;
	setItem(key: string, value: string): void;
	removeItem?(key: string): void;
}

const keyOf = (storyboard: string) => `sc-storyboard-slots:${storyboard}`;
const isPick = (v: unknown): v is SlotPick =>
	!!v && typeof v === 'object' && (typeof (v as { file?: unknown }).file === 'string' || typeof (v as { generator?: unknown }).generator === 'string');

/** the picks of a storyboard's shots kept in this browser (anything unreadable is dropped) */
export function readSlots(store: Store | null, storyboard: string): Record<string, SlotPick> {
	try {
		const raw = JSON.parse(store?.getItem(keyOf(storyboard)) ?? 'null');
		if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return {};
		return Object.fromEntries(Object.entries(raw).filter((e): e is [string, SlotPick] => isPick(e[1])));
	} catch {
		return {};
	}
}

/** a shot's pick set (null empties the slot); → the storyboard's picks after it */
export function writeSlot(store: Store | null, storyboard: string, slots: Readonly<Record<string, SlotPick>>, shot: string, pick: SlotPick | null): Record<string, SlotPick> {
	const next = { ...slots };
	if (pick) next[shot] = pick;
	else delete next[shot];
	try {
		if (Object.keys(next).length) store?.setItem(keyOf(storyboard), JSON.stringify(next));
		else if (store?.removeItem) store.removeItem(keyOf(storyboard));
		else store?.setItem(keyOf(storyboard), '{}');
	} catch {
		// storage blocked: the pick holds for this visit
	}
	return next;
}

/** what a shot's slot shows: its pick, else its approved generation with a file, else nothing */
export function slotContent(pick: SlotPick | undefined, approved: { id: number; file: string | null; kind?: string } | undefined): SlotContent {
	if (pick && 'file' in pick) return { at: 'file', path: pick.file, video: kindOf(pick.file) === 'video' };
	if (pick && 'generator' in pick) return { at: 'generator', address: pick.generator, name: pick.name || pick.generator.replace(/^generator:/, '') };
	if (approved?.file) return { at: 'approved', generation: approved.id, video: (approved.kind ?? kindOf(approved.file)) === 'video' };
	return { at: 'empty' };
}
