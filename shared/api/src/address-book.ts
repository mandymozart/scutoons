// GENERATED from server/schema/api/address-book.schema.json by server/schema/generate.ts – edit the source, run `npm run generate`.
// One toon's address book (#477, decisions/deck.md [contract] [typeahead]): every addressable thing of the toon's catalog with its address, kind, labels and where it comes from. The server writes it (GET /api/toons/<toon>/address-book), the creator's VN events and the deck editor filter it in one typeahead each; whether an address is valid stays the core's on save (/validate). Edit this file, run `npm run generate` (server/schema/generate.ts writes the Rust, TS and PHP types), commit the outputs.
/** What an entry is – the one thing a typeahead filters by: an asker takes the kinds it can address (a VN character event: character, persona, look; unlockLocation: location; an item event: item; a jump: chapter). character / persona / look are the levels of address.rs's `Target` (`character:<id>[.<persona>[.<look>]]`); the others are a place `<kind>:<id>`. */
export type AddressKind = 'character' | 'persona' | 'look' | 'location' | 'item' | 'chapter';

/** `published`: the newest version of the origin document is the published one; `draft`: a newer draft exists (or nothing is published yet), and the entry is the draft's. */
export type OriginState = 'published' | 'draft';

/** Content text: locale → text (the one shape of authored text; pick a locale with the one locale order, server/schema/tables/locale-order.json). */
export type LocaleText = Record<string, string>;

/** The catalog document an entry comes from. */
export interface Origin {
	/** the document's entity key (`character:catpope`, `place:dom`, `chapter:catpope:3`) or catalog entry (`item:incense`) */
	key: string;
	state: OriginState;
}

/** One addressable thing. */
export interface AddressEntry {
	/** the address, in address.rs's grammar: `character:catpope.human.default`, `location:dom`, `item:incense`, `chapter:3` */
	address: string;
	kind: AddressKind;
	/** its own id at its level (`default` for the look above, `3` for a chapter): what a document's field holds */
	name: string;
	/** the address one level up (a look's persona, a persona's character); absent at the top */
	parent?: string;
	labels: LocaleText;
	origin: Origin;
	/** a look only: the moods it has art for, in the document's order */
	moods?: string[];
}

/** The document: one per toon, versioned by the catalog it was built from. */
export interface AddressBook {
	/** the format's version */
	address_book: 1;
	toon: string;
	/** the catalog's version it was built from (the response's ETag) */
	version: string;
	/** the toon's source language: the second step of the locale order */
	source: string;
	/** parents before their children; characters, locations, items, chapters */
	entries: AddressEntry[];
}
