// GENERATED from server/schema/content/scene.schema.json by server/schema/generate.ts – edit the source, run `npm run generate`.
// The stored scene document `scene:<id>` (#243, research/SCENES.md §3): deck-space's SceneDocument (deck/space/src/scene_document.rs) →
// its JSON Schema (deck-schema) → these types. Whether a scene plays is the core's (the build service's /validate at publish).

/** the format the API stores and the decks read */
export const SCENE_FORMAT = 2 as const;

/** A spawn zone's arrangement: its kind and the spacing between members. */
export type Arrange = {
	kind: ArrangeKind;
	/** metres between neighbours (a set's spot is a point: the spacing says how wide the zone grows) */
	spacing?: number;
};

/** The arrangements of a first version (scene_resolve.rs `arrange`): more (a stack, a crowd) when a use needs one. */
export type ArrangeKind = 'row' | 'cluster';

/** `start` or `join`. */
export type Enter = 'start' | 'join';

/** The filter of a placeholder (SCENES.md §9.2): the address book's typeahead question, stored. Within a fact any value admits; every fact given must hold (owner 2026-11-12, §9.7 Q5). */
export type Fill = {
	/** the address book's kinds that may fill it (required): a holder's of `character`, `persona`, `look`, `item`; a location's `location` */
	kind: ('character' | 'persona' | 'look' | 'item' | 'location')[];
	/** only entries below this reference (`character:catpope`: her personas and looks) */
	parent?: string | null;
	/** a hand-picked list of references (`character:catpope`, `item:incense`); none = any of the kinds */
	ref?: string[];
};

/** One filler or several (a spawn zone's members). */
export type Fillers = string | string[];

/** One place on the stage (SCENE-PACKAGE §2, SCENES.md §9.3). */
export type Holder = {
	/** a spawn zone's arrangement of its members within the spot (required with `take: many`) */
	arrange?: Arrange | null;
	/** `start` (the default): on stage when the scene starts; `join`: waits for the story (`joinScene`, SCENES.md §9.4) */
	enter?: Enter;
	/** a placeholder's filter – or `ref` */
	fill?: Fill | null;
	/** its name in the document and in a use's `with` (`left`, `who`); not an address – a filled holder's address is its filler's place (`scene:stage/character:catpope/…`), an empty one has none */
	id: string;
	/** a placeholder's name for people */
	label?: Record<string, string> | null;
	/** a spawn zone's limit; none = any number */
	max?: number | null;
	/** fillers for the editor and the browser (a spawn zone may preview several) */
	preview?: Fillers | null;
	/** an authored filler (`character:catpope`, `item:incense`) – or `fill`, never both */
	ref?: string | null;
	/** the rig its filler plays; none = the filler's own (`character:catpope` → `rig:character.catpope`) */
	rig?: RigPin | null;
	/** the spot of the set where it stands and enters (`spot_left`); none = the scene's spot */
	spot?: string | null;
	/** `one` filler (the default) or `many`: a spawn zone (owner 2026-11-12 [scenes]) – a placeholder only */
	take?: Take;
	/** values and preset choices only (SCENE-PACKAGE §2), by address: `"voice:voice.index": 4.0`, `"voice:@preset": "cathedral_voice.bright"` */
	values?: Record<string, HolderValue>;
};

/** A holder's value: a parameter's number, or a track's preset choice. */
export type HolderValue = number | string;

export type LinkDef = {
	from: string;
	/** an audio link's channel map (plan 2.6, #318): `[source, target]` pairs counted from 1 – `[[1, 3], [2, 4]]` lands a stereo source on channels 3 and 4 (a send's offset); one source may go to several targets, several sources into one target sum. Empty = channel n to channel n. `scale` on an audio link is the send level. */
	map?: [number, number][];
	offset?: number;
	scale?: number;
	to: string;
};

/** A location no scene names yet: what may fill it. */
export type LocationPlaceholder = {
	/** the filter: `kind` is `["location"]` */
	fill: Fill;
	/** its name in a use's `with` (`where`, `home`); not an address */
	id: string;
	/** its name for people */
	label?: Record<string, string> | null;
	/** a filler for the editor and the browser */
	preview?: string | null;
};

/** The scene's location: a reference, or a placeholder. */
export type LocationSlot = string | LocationPlaceholder;

export type NodeDef = {
	id: string;
	kind: string;
	[key: string]: unknown;
};

/** A rig by reference and the version it was wired against (SCENE-PACKAGE §2). */
export type RigPin = {
	/** `rig:<subject>` (`rig:character.catpope`) */
	ref: string;
	/** the version it was wired against: the rig's `version` as scene-content serves it (16 hex of its data's SHA-256 – the catalog keeps no version number); none = the current one */
	version?: string | null;
};

/** What a switch does to the outputs (the lamps) between two scenes (#264). */
export type SceneTransition = 'fade' | 'cut' | 'blackout';

export type StageSettings = {
	/** the audio device's buffer in frames (#208, deck-native's audio/backend.rs): a heavy scene may ask for more; `--audio-buffer` and deck.toml's `audio_buffer` (the machine's) win over it. None = the device's default */
	audio_buffer?: number | null;
	name?: string;
	/** the render size relative to the window (0.25–1); a software adapter halves it by itself */
	render_scale?: number;
	/** how the stage hands over TO this scene on a switch (F5, #264): the DMX / MIDI outputs of the scene that was playing fade into this one's (`fade`, the default), snap (`cut`) or go dark first (`blackout`); a stop is always dark */
	transition?: SceneTransition;
	/** the hand-over's length in s (`fade`); the same time base as the picture's fade in (deck-native switcher.rs `FADE_IN`) */
	transition_secs?: number;
};

/** How many fillers a holder takes. */
export type Take = 'one' | 'many';

/** One `scene:<id>` document as the API stores it. */
export type SceneDocument = {
	/** the document's format: 2 (location, holders, spawn zones; #489) */
	format: 2;
	/** the places on its stage, in the order a story's `joinScene` tries them (SCENES.md §9.4) */
	holder?: Holder[];
	/** the key's id: `nave` for `scene:nave` – platform-wide, no toon in it (a scene can serve several toons and shows) */
	id: string;
	link?: LinkDef[];
	/** where it plays: `location:<id>`, or a placeholder a use fills; none = the `space` node's own `set` (or no set) */
	location?: LocationSlot | null;
	/** the scene's own nodes (the space, mixers); the holders' rigs are flattened in by the resolve step */
	node?: NodeDef[];
	/** the parameters as outputs (#211, scene.rs `Scene::outputs`) */
	outputs?: Record<string, unknown> | null;
	stage?: StageSettings;
	/** the toons whose address books list it; empty = reachable by its id only (a template: listed in every book, 2·2) */
	toons?: string[];
	/** the number the server stamps on every change (as a show's); absent until the first save */
	version?: number | null;
};
