// rig.ts (#191, #368): the core's answers about what a deck plays, as JSON types – no rule. The rig is played, replayed on
// a mood, diffed and its facts (streamed paths, knob targets, sticks) are worked out in Rust once (deck-space's rig and
// deck modules, space-web `DeckState`); the page applies the turns (deck.ts). #282: what plays, wired (wiring.ts).
export type { RigWiring, WireLink, WireNode, WirePort, WireType } from './wiring.ts';
/** how a streamed path moves (deck-space `rig::Stream`; its epsilon is the core's) */
export type PathKind = { input: [min: number, max: number] } | 'audio' | 'control' | 'shape';

/** a knob of a played scene (deck-space `rig::Knob`): an exposed parameter of the flattened scene */
export interface RigKnob {
	/** `<track>_<node>.<port>` */
	path: string;
	label?: Record<string, string>;
	min: number;
	max: number;
	value: number;
	log?: boolean;
}

/** a scene as the core plays it – a character's rig for a persona and a mood, or the deck's own (deck-space `rig::Played`) */
export interface RigPlayed {
	/** a stage scene in TOML (its space node bare) */
	scene: string;
	nodes: { id: string; kind: string; [param: string]: unknown }[];
	links: { from: string; to: string; scale: number; offset: number }[];
	/** the head's character; empty for an item's rig */
	character: string;
	knobs: RigKnob[];
	tracks: { track: string; preset?: string }[];
	/** the corner sticks' knob paths, (x, y) left then right ("" = none) */
	sticks: [[string, string], [string, string]];
	/** what live-core plays of it (deck-space's `lower`: deck-core's carried part for the browser's modules) */
	engine: {
		scene: string;
		/** the nodes of modules the browser does not carry yet (chapter_music, module:*) – not played */
		page: { id: string; kind: string; [param: string]: unknown }[];
		dropped: { from: string; to: string; scale: number; offset: number }[];
	};
	/** every path the deck streams, with how it moves */
	streamed: [string, PathKind][];
	/** the knobs the being shows */
	expose: string[];
}

/** what changed in what the deck plays (deck-space `deck::Turn`): a scene to load whole, or the values a mood (or a code
 *  rig's preset choice, #410) moved */
export interface RigTurn {
	why: 'play' | 'mood' | 'preset';
	/** a rig plays (false: the deck's own scene) */
	rig: boolean;
	reload: boolean;
	values: [string, number][];
	played: RigPlayed;
	/** for the log: which rig, or why the deck's own scene plays */
	note: string;
	/** the rig failed, the deck's own scene plays */
	failed: boolean;
}

/** a frame's answer when the mood turned (deck-space `deck::Step`) */
export interface MoodStep {
	mood: string;
	turn?: RigTurn;
	error?: string;
}

/** a knob as a slider shows it (deck-space `deck::Slider`) – the page's `Spec` with its path */
export interface RigSlider {
	path: string;
	min: number;
	max: number;
	value: number;
	log?: boolean;
	label: string;
}
