// A slot's pick (lap2 C12c, LIBRARY.md "A picker is the Library opened from a slot"): the place shown first – what this
// thing already has – asked beside the picker's list (pickShelf), and what a press on an entry does. The server answers
// what is listed; the core says whether a file fits (library.pickFile); this only holds and routes.
import { libraryEntries, libraryPath, type LibraryEntry, type LibraryLevel, type LibraryQuery } from '@scutoons/api';
import { api } from '$lib/api.ts';
import { isThingPick, library, type ThingPick } from '$lib/editor/library.svelte.ts';
import { pickShelf } from '$lib/editor/library-shelf.svelte.ts';
import { misfitWords } from '$lib/editor/library-drop.ts';
import { filePath, levelIds, thingId } from '$lib/editor/library-list.ts';
import { shared } from '$lib/utils/shared.ts';

class PickFirst {
	entries = $state.raw<LibraryEntry[]>([]);
	#key: string | null = null;

	/** the first place's entries (none: no first place) */
	async show(q: LibraryQuery | null) {
		const key = q ? libraryPath(q) : null;
		if (key === this.#key) return;
		this.#key = key;
		this.entries = [];
		if (!q) return;
		try {
			const entries = await libraryEntries(api, q);
			if (this.#key === key) this.entries = entries;
		} catch {
			// the list below still answers; the first place stays empty
		}
	}
}

export const pickFirst = shared('creator-library-pick-first', () => new PickFirst());

/** whether the slot takes this thing as it is (its type, not taken, accepted) */
export function takesThing(entry: LibraryEntry): boolean {
	const request = library.pick;
	if (!isThingPick(request) || !entry.type || !request.types.includes(entry.type)) return false;
	return !request.taken?.includes(entry.address) && (!request.accept || request.accept(entry));
}

/** a press on an entry: a file fills its slot (or says why not), a thing the slot takes is picked – or stepped into when the slot wants one of its levels –, any other thing is stepped into. → a note to show, or '' */
export function pressEntry(entry: LibraryEntry, deeper = false): string {
	const request = library.pick;
	if (!request) return '';
	const path = filePath(entry);
	if (path) {
		const answer = library.pickFile(path);
		return answer.ok ? '' : misfitWords(answer);
	}
	if (takesThing(entry) && isThingPick(request) && !request.depth) {
		library.pickThing({ address: entry.address, entry });
		return '';
	}
	pickShelf.stepInto(entry.address, deeper);
	return '';
}

/** Pick in a thing stepped into: the thing as deep as its levels were chosen */
export function pickStepped(levels: LibraryLevel[]) {
	const at = pickShelf.step?.at;
	if (at) library.pickThing({ address: at.address, entry: at, ...levelIds(levels) });
}

/** an item slot asks: plain catalog items (no recipe, product …); the pick hands the item's id */
export function pickItem(request: Omit<ThingPick, 'types' | 'accept' | 'onthing'>, onpick: (id: string) => void) {
	library.startPick({ ...request, types: ['item'], accept: (e) => !e.subtype, onthing: (picked) => onpick(thingId(picked.address)) });
}
