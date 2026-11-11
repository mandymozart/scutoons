// GENERATED from server/schema/content/scene.schema.json by server/schema/generate.ts – edit the source, run `npm run generate`.
// The stored scene document `scene:<id>` (#243, research/SCENES.md §3): deck-space's SceneDocument (deck/space/src/scene_document.rs) →
// its JSON Schema (deck-schema) → these types. Whether a scene plays is the core's (the build service's /validate at publish).

/** the format the API stores and the decks read */
export const SCENE_FORMAT = 1 as const;

export type LinkDef = {
	from: string;
	/** an audio link's channel map (plan 2.6, #318): `[source, target]` pairs counted from 1 – `[[1, 3], [2, 4]]` lands a stereo source on channels 3 and 4 (a send's offset); one source may go to several targets, several sources into one target sum. Empty = channel n to channel n. `scale` on an audio link is the send level. */
	map?: [number, number][];
	offset?: number;
	scale?: number;
	to: string;
};

/** The scene's `[being]` table – the body's shape (the colours are the space node's inputs). */
export type Look = {
	/** how much its slow breath swells it (a fraction of its size) */
	breath?: number;
	/** bubble rate × this */
	bubbles?: number;
	/** the head's character (its portrait sprites `<character>_portrait-<mood>`); empty = a head without a face */
	character?: string;
	/** how far it floats around its spot (m) */
	drift?: number;
	/** the parameters shown as buttons: "node" (all its inputs) or "node.port"; empty = every control input – or a counted group `"seq.step1..16 <= seq.steps"` (#424, params.rs `Expose`): the first N of those knobs, N = the value of `seq.steps` now. The list's order is the knobs' order on a node (the first named hangs nearest its point). */
	expose?: string[];
	/** feedback → emotion: `[[being.feel]]` rules (feel.rs) */
	feel?: Rule[];
	/** hose thickness × this */
	hose?: number;
	/** the moods the energy walks through, calm → wild (default MOODS) */
	moods?: string[];
	/** emotion → sound: `[[being.react]]` nudges (feel.rs) */
	react?: Nudge[];
	/** the body's radius (m) */
	size?: number;
	/** each point's own small sway (a fraction of the size) */
	wobble?: number;
};

export type NodeDef = {
	id: string;
	kind: string;
	[key: string]: unknown;
};

/** One `[[being.react]]` nudge as the scene writes it. */
export type Nudge = {
	attack?: number;
	by?: number | null;
	/** an unlinked control input, `node.port` (by id, else by kind). Was `set` until #243 (SCENES.md §8.2: in a scene `set` is the staging only); the repo's scenes were migrated, no alias (RULES E2) */
	input?: string;
	mood?: string;
	release?: number;
	to?: number | null;
};

/** One `[[being.feel]]` rule as the scene writes it. */
export type Rule = {
	above?: number | null;
	below?: number | null;
	by?: number;
	/** 0..1: the odds that it fires when its test comes true */
	chance?: number;
	cooldown?: number;
	falling?: number | null;
	/** seconds the test must hold before the rule fires */
	for?: number;
	hold?: number;
	mood?: string;
	rising?: number | null;
	/** seconds the source stays within `by` */
	steady?: number | null;
	weight?: number;
	/** the source: `node.port` (the node by id, else the first of that kind), `bpm`, `touch`, `energy`, `phrase` */
	when?: string;
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

/** One `scene:<id>` document as the API stores it: the header, then the scene's own fields at the top level (`stage`, `node`, `link`, `outputs`, `being`). */
export type SceneDocument = {
	being?: Look;
	/** the document's format: 1 (today's scene graph, no holders) */
	format: 1;
	/** the key's id: `nave` for `scene:nave` – platform-wide, no toon in it (a scene can serve several toons and shows) */
	id: string;
	link?: LinkDef[];
	node?: NodeDef[];
	/** the parameters as outputs (#211) – MIDI out, the conductor, DMX fixtures and the clock-synced strobe; parsed and checked by deck-outputs (its config.rs documents the table), played by the host (deck-native's outputs.rs) */
	outputs?: Record<string, unknown> | null;
	stage?: StageSettings;
	/** the toons whose address books list it; empty = reachable by its id only (proposal, SCENES.md §7 question 5) */
	toons?: string[];
	/** the number the server stamps on every change (as a show's); absent until the first save */
	version?: number | null;
};
