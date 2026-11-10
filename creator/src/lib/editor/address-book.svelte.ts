// The toons' address books in the creator (#477 slice B): fetched once per page visit through @scutoons/api, refetched
// with the version held (`If-None-Match`, a 304 keeps it) when a typeahead opens. A failed fetch keeps the book held
// (marked `error`); without one the field falls back to plain text (address-field.svelte).
import type { AddressBook } from '@scutoons/api';
import { contentApi } from '$lib/api.ts';
import { shared } from '$lib/utils/shared.ts';

export interface HeldBook {
	/** null = none yet (loading, or the first fetch failed) */
	book: AddressBook | null;
	loading: boolean;
	/** the last fetch failed (offline, no access …) */
	error: boolean;
}

/** a refetch at most this often per toon (opening the field again and again asks once) */
const REFETCH_MS = 3000;

class AddressBooks {
	#held = $state.raw<Record<string, HeldBook>>({});
	#pending = new Map<string, Promise<void>>();
	#asked = new Map<string, number>();

	/** the toon's book as held now; the first call starts the fetch */
	of(toon: string): HeldBook {
		const held = this.#held[toon];
		if (!held && toon) queueMicrotask(() => void this.refresh(toon));
		return held ?? { book: null, loading: true, error: false };
	}

	/** fetch it again (the version held → 304 when unchanged); one at a time per toon */
	refresh(toon: string): Promise<void> {
		const pending = this.#pending.get(toon);
		if (pending) return pending;
		if (Date.now() - (this.#asked.get(toon) ?? -Infinity) < REFETCH_MS) return Promise.resolve();
		this.#asked.set(toon, Date.now());
		const held = this.#held[toon];
		this.#set(toon, { book: held?.book ?? null, loading: true, error: false });
		const run = contentApi
			.addressBook(toon, held?.book?.version)
			.then((book) => this.#set(toon, { book: book ?? this.#held[toon]?.book ?? null, loading: false, error: false }))
			.catch(() => this.#set(toon, { book: this.#held[toon]?.book ?? null, loading: false, error: true }))
			.finally(() => this.#pending.delete(toon));
		this.#pending.set(toon, run);
		return run;
	}

	#set(toon: string, held: HeldBook) {
		this.#held = { ...this.#held, [toon]: held };
	}
}

export const addressBooks = shared('address-books', () => new AddressBooks());
