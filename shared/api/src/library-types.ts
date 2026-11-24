// GENERATED from server/schema/api/library.schema.json by server/schema/generate.ts – edit the source, run `npm run generate`.
// The extended Library (#864, agents/research/redesign/lap2/LIBRARY.md): things and files in one list, one server answer for the creator and the deck (GET /api/library, server/docs/API.md "The extended Library"). The server resolves – types, kinds, names, pictures, places, used in – the apps only read. Edit this file, run `npm run generate` (server/schema/generate.ts writes the TS and PHP types), commit the outputs; library.vectors.json holds the file cases every reader holds to.

/** the format the API answers (`library`) */
export const LIBRARY_FORMAT = 1 as const;

/** What a thing is (owner 2026-11-24, LIBRARY.md "Answered" 1): a toon, a character, a deck scene, a live show, a place of the world map, a catalog item (with its subtype), a rig module, an instrument, a rig, a generator (the storyboard's providers), and for now a Kanon text and a person (for credits). Its address is its entity key (`toon:catpope`, `character:catpope`, `scene:nave`, `show:duet`, `place:dom`, `item:camera`, `recipe:x`, `product:x`, `edition:x`, `album:x`, `home-scene:x`, `module:walk`, `instrument:organ`, `rig:character.catpope`, `kanon:character.catpope:de`), a generator's `generator:<id>`, a person's `person:<id>`. */
export type ThingType = 'toon' | 'character' | 'scene' | 'show' | 'place' | 'item' | 'module' | 'instrument' | 'rig' | 'generator' | 'kanon' | 'person';

/** An item's subtype: a catalog entry of its own table listed as an item – absent on a plain item (`item:<id>`). */
export type ItemSubtype = 'recipe' | 'product' | 'edition' | 'album' | 'home-scene';

/** What a file is: the registry's kind, where `archive`, `document`, `font` and `other` are a `file`. Its address is `<kind>:<path>` – the kind and the current version's main file (CDN-root path), as the Library opens it. */
export type FileKind = 'image' | 'audio' | 'video' | 'model' | 'animation' | 'file';

/** Where a file came from (the From place): its main file's origin in the registry. */
export type FileOrigin = 'uploaded' | 'imported' | 'generated';

/** How a thing uses another: `cast` (a season's cast lists the character), `credits` (a toon's credits name the person), `rig` (a character or item plays this rig; a rig plays this instrument or module), `reference` (the document names it: an address in a scene or show, a character or item in a chapter's events, an item in a recipe or product, a file in a document). */
export type LinkRole = 'cast' | 'credits' | 'rig' | 'reference';

/** A level inside a thing: a character's persona › look › mood, a toon's season › chapter. */
export type LevelKind = 'persona' | 'look' | 'mood' | 'season' | 'chapter';

/** One entry: a thing (with `type`) or a file (with `kind`), never both. */
export type LibraryEntry = {
	/** the identity: a thing's address (ThingType) or a file's `<kind>:<path>` */
	address: string;
	type?: ThingType;
	subtype?: ItemSubtype;
	kind?: FileKind;
	/** what a person reads: the label in the asked locale (the one locale order), else the id; a file's title, else its slug */
	name: string;
	/** a thumbnail's URL (the CDN's): a file's smallest picture (an image's, a video's poster), a thing's picture (a character's default look, a toon's tile, an item's icon, a place's image) – null where there is no still yet (a model, a scene, a module) */
	picture: string | null;
	/** the thing it belongs to: a rig's character or item, a Kanon text's subject */
	parent?: string;
	/** the toons it belongs to (the place This toon); empty = shared by every toon (an instrument, a template scene, a file no toon uses) */
	toons: string[];
	/** how many things use it (a file: how many documents; a character: its casts, scenes, shows, chapters …) */
	usedIn: number;
	/** when it last changed (ISO 8601, UTC): a thing's newest version, a file's registry entry – the place Recently added */
	updated: string | null;
	origin?: FileOrigin;
};

/** How many entries of each type and kind the place holds before the type / kind filter (the type select's numbers): `character`, `image`, … */
export type LibraryCounts = Record<string, number>;

/** GET /api/library[?toon=&place=&type=&subtype=&kind=&from=&locale=&cursor=&limit=]: a page of one place's entries – things first (in ThingType's order), then files (in FileKind's order), each by address. */
export type LibraryList = {
	/** the format's version */
	library: 1;
	/** the answer's version (the ETag without quotes) */
	version: string;
	/** the toon asked for; null = the universe */
	toon: string | null;
	/** the place: `toon`, `all`, `recent`, `unused`, `item:<address>`, `cast:<season>` */
	place: string;
	locale: string;
	entries: LibraryEntry[];
	/** the entries the place and the filters hold, every page together */
	total: number;
	/** the cursor of the next page; null = the last page */
	next: string | null;
	counts: LibraryCounts;
};

/** One level inside a thing, parents before children. A mood has no address of its own (the address grammar ends at the look, space rig/address.rs): its `address` is its look's, `mood` names it. */
export type LibraryLevel = {
	address: string;
	level: LevelKind;
	name: string;
	picture: string | null;
	/** the level (or the thing) one up */
	parent: string;
	/** a mood only: its id */
	mood?: string;
};

/** A thing that uses the stepped-into one ("used in"): a link to open. */
export type LibraryLink = {
	/** its address (a season's cast: `season:<toon>:<n>`, a chapter: `chapter:<toon>:<n>` – documents that are no entry of the list are linked too) */
	address: string;
	type?: ThingType;
	name: string;
	as: LinkRole;
};

/** GET /api/library?at=<address>[&locale=]: a thing stepped into – its levels, the files it uses, the things it uses, the things that use it; a file's `at` answers its entry and what uses it. */
export type LibraryStep = {
	library: 1;
	version: string;
	locale: string;
	at: LibraryEntry;
	levels: LibraryLevel[];
	/** the files it uses (the place This item's files) */
	files: LibraryEntry[];
	/** the things it uses */
	uses: LibraryEntry[];
	usedIn: LibraryLink[];
};

/** every ThingType, in the schema's order */
export const THING_TYPES: readonly ThingType[] = ['toon', 'character', 'scene', 'show', 'place', 'item', 'module', 'instrument', 'rig', 'generator', 'kanon', 'person'];

/** every ItemSubtype, in the schema's order */
export const ITEM_SUBTYPES: readonly ItemSubtype[] = ['recipe', 'product', 'edition', 'album', 'home-scene'];

/** every FileKind, in the schema's order */
export const FILE_KINDS: readonly FileKind[] = ['image', 'audio', 'video', 'model', 'animation', 'file'];

/** every FileOrigin, in the schema's order */
export const FILE_ORIGINS: readonly FileOrigin[] = ['uploaded', 'imported', 'generated'];

/** every LinkRole, in the schema's order */
export const LINK_ROLES: readonly LinkRole[] = ['cast', 'credits', 'rig', 'reference'];

/** every LevelKind, in the schema's order */
export const LEVEL_KINDS: readonly LevelKind[] = ['persona', 'look', 'mood', 'season', 'chapter'];
