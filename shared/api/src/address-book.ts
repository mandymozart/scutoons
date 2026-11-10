// GENERATED from server/schema/api/address-book.schema.json by server/schema/generate.ts – edit the source, run `npm run generate`.
// One toon's address book (#477, decisions/deck.md [contract] [typeahead] [address]): every thing of the toon's catalog a typeahead may pick, with its reference, its address where something can receive one, its kind, labels and where it comes from. The server writes it (GET /api/addresses/<toon>), the creator's VN events and the deck editor filter it in one typeahead each; whether an address is valid stays the core's on save (/validate). Edit this file, run `npm run generate` (server/schema/generate.ts writes the Rust, TS and PHP types), commit the outputs.
/** What an entry is – the one thing a typeahead filters by: an asker takes the kinds it may pick (a VN character event: character, persona, look; unlockLocation: location; an item event: item; a jump: chapter, group, season, toon, product; setVariable: variable). A jump's kinds are its target types (`JumpToTargetType`, a group is a chapter target with its label): `season:<number>` (the toon's season), `toon:<id>` (a toon: its page, and in the creator's jump field the way into its own book), `product:<id>` (a shop product of the toon or a shared one); XR is not listed: there is no catalog of XR targets. character / persona / look are the levels of address.rs's `Target` (`character:<id>[.<persona>[.<look>]]`); location, item and chapter are referenced `<kind>:<id>`. Proposals the owner has not confirmed (#477): a chapter's group is referenced `chapter:<number>.<group>` (its `name` the label as an event stores it, the reference word a safe form of it) and a story variable `variable:<scope>.<name>` (scope = the owning toon or `global`; its `name` the variable as written in this toon). */
export type AddressKind = 'character' | 'persona' | 'look' | 'location' | 'item' | 'chapter' | 'group' | 'variable' | 'season' | 'toon' | 'product';

/** `published`: the newest version of the origin document is the published one; `draft`: a newer draft exists (or nothing is published yet), and the entry is the draft's. */
export type OriginState = 'published' | 'draft';

/** Content text: locale → text (the one shape of authored text; pick a locale with the one locale order, server/schema/tables/locale-order.json). */
export type LocaleText = Record<string, string>;

/** The catalog document an entry comes from. */
export interface Origin {
	/** the document's entity key (`character:catpope`, `place:dom`, `chapter:catpope:3`, `season:catpope:1`, `toon:catpope`) or catalog entry (`item:incense`, `product:dago_hoodie`) */
	key: string;
	state: OriginState;
}

/** One thing of the catalog: its reference always, its address only where a receiver of its kind exists (decisions/deck.md [address]: there is no interpreter, sending a word somewhere needs a possible receiver of the same type). */
export interface AddressEntry {
	/** the reference – which thing; every entry has one, stable; what a typeahead picks and a document's fields are filled from: `character:catpope.human.default`, `location:dom`, `item:incense`, `chapter:3`, `chapter:3.intro`, `season:1`, `toon:toptoy`, `product:dago_hoodie`, `variable:global.coins` */
	ref: string;
	/** the address – who hears a word, in address.rs's grammar, the same text as `ref`; only where a receiver of the kind exists: character and item (deck-space far.rs), persona and look (the show's addressing, `picks_up`). A location, chapter, group or variable has none: nothing receives a word sent there today. */
	address?: string;
	kind: AddressKind;
	/** its own id at its level (`default` for the look above, `3` for a chapter, a group's label, a variable as written in this toon: `coins`, `global.coins`, `toptoy.coins`): what a document's field holds */
	name: string;
	/** the reference one level up (a look's persona, a persona's character); absent at the top */
	parent?: string;
	labels: LocaleText;
	origin: Origin;
	/** a look only: the moods it has art for, in the document's order */
	moods?: string[];
}

/** The document: one per toon, versioned by the catalog it was built from. */
export interface AddressBook {
	/** the format's version (2: `ref` on every entry, `address` only where a receiver exists) */
	address_book: 2;
	toon: string;
	/** the catalog's version it was built from (the response's ETag) */
	version: string;
	/** the toon's source language: the second step of the locale order */
	source: string;
	/** parents before their children; characters, locations, items, seasons, chapters (each followed by its groups), products, toons (this one first), variables */
	entries: AddressEntry[];
}
