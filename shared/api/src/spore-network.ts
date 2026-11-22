// GENERATED from server/schema/api/spore-network.schema.json by server/schema/generate.ts – edit the source, run `npm run generate`.
// One level of the spore network as a weave (#763, agents/research/redesign/NETWORK.md §4; owner 2026-11-21: the tab is "Spores", connecting is "weaving"): the spores and threads a scope's projector computes from its documents, drawn by every canvas – the creator's Spores tab, the native deck editor, the Reception tab, the MCP. The documents stay the truth; a weave is never stored (only its canvas). Each scope's projector lives where that scope's rules live (story: shared/content; XR: the OSCT logic package; show and rig: the Rust core). Edit this file, run `npm run generate` (server/schema/generate.ts writes the Rust and TS types), commit the outputs; spore-network.vectors.json holds the weaves every language reads.
/** What travels on a port or thread – deck-kit's one signal table (deck/kit/src/base/signal.rs, owner 2026-11-17 [design]): its shapes and colours draw every port and thread. OSCT's `value` is `control`, its `node` is `address`, its `media` is `picture`. `job` is a run that starts, progresses and ends (a generation, a night run, a publish); `asset` is a reference to a Library asset of any kind (a picture that arrived, an approved text) – owner 2026-11-21: spores get job and asset signals. */
export type Signal = 'event' | 'control' | 'clock' | 'audio' | 'picture' | 'address' | 'vote' | 'scene' | 'far' | 'job' | 'asset';

/** Which documents a weave projects: `story` chapters and toons, `xr` experiences, `show` rows and cells, `rig` rigs and instruments, `scene` deck scenes, `creator` the creator's own events that can trigger spores (a night run, a publish, a draft saved, a job triggered, a picture arriving in another spore, a budget authorized, a Kanon text or script approved, a storyboard cut, a chapter's status changed – owner 2026-11-21), `network` the spore network's levels above them all. */
export type Scope = 'story' | 'xr' | 'show' | 'rig' | 'scene' | 'creator' | 'network';

/** The driver that runs a spore (the runtimes): the `reader` (the VN runner), `xr` (OSCT's engine in the reader), the `conductor`, a `deck` (the engine, its own audio context), the `server` (jobs, generations), the `editor` (authoring-time nodes only, never at reading time). */
export type Driver = 'reader' | 'xr' | 'conductor' | 'deck' | 'server' | 'editor';

/** A spore's family – how a canvas draws it and what a gesture may do. `container` can be entered (a chapter, a group, a row, a cell, a track); `world` comes from outside the level with an image of its origin (MIDI, clock, carrier bag, VN state); `free` lives only in the canvas (a reference, a generation, a manual instruction – OSCT L11–L13); `far` is a stub's far end. */
export type Family = 'event' | 'rule' | 'action' | 'variable' | 'entity' | 'scene' | 'plugin' | 'container' | 'sound' | 'world' | 'free' | 'far';

export type Direction = 'in' | 'out';

/** How a thread crosses runtimes: `same` (one runtime), `relay` (the Live relay), `navigation` (the reader moves to another place), `savegame` (a variable or item read elsewhere). */
export type Transport = 'same' | 'relay' | 'navigation' | 'savegame';

/** What a port connects to: `cyber` (devices, modules, the engine – called a port) or `organic` (beings, characters, living matter – called a pore). Only the word and the look differ; the signal decides what may connect. */
export type Matter = 'cyber' | 'organic';

/** One step of the breadcrumb, root first. */
export interface Level {
	/** The level's address in the one grammar (rig/address.rs), e.g. `chapter:sandbox/1/2`. */
	address: string;
	label: string;
}

export interface Port {
	id: string;
	label: string;
	signal: Signal;
	dir: Direction;
	/** Several edges may meet here (a multi-in socket). */
	many?: boolean;
	/** Unused and folded behind the node's `+`. */
	hidden?: boolean;
	matter?: Matter;
}

/** A spore: one thing of a level – a group, a rule, a cell, a module, a generator. Its `kind` is its spore type, registered by a scope's projector, a plugin, a component or a module. */
export interface Spore {
	/** Unique within the view. */
	id: string;
	/** The spore type. */
	kind: string;
	family: Family;
	label: string;
	detail?: string;
	/** What the spore is, in the one grammar. */
	address: string;
	driver: Driver;
	ports: Port[];
	/** The address of the level a double-click opens; none: a leaf. */
	enter?: string;
	/** An asset reference for the spore's image (a world spore's origin, a preview). */
	picture?: string;
}

/** One end of a thread: a spore and one of its ports. */
export interface End {
	spore: string;
	port: string;
}

/** The one document field that holds a thread or a spore: an edit is an op on it. None on a thread: derived (a by-name join, a computed link). */
export interface Home {
	/** The document's address, e.g. `rig:crow`, `chapter:sandbox/1/2`, `show:demo`. */
	doc: string;
	/** A JSON pointer into the document. */
	path: string;
}

/** A thread woven between two spores' ports. */
export interface Thread {
	id: string;
	from: End;
	to: End;
	signal: Signal;
	label?: string;
	home?: Home;
	crosses?: Transport;
	/** A bundle of this many threads of one kind between one pair. */
	count?: number;
}

/** A thread that leaves the level, named by its far end; following it changes the weave's scope. */
export interface Stub {
	/** The thread's id; its spore at the level's edge is a `far` spore. */
	thread: string;
	side: Direction;
	/** The far end's address. */
	far: string;
	label: string;
}

export type Severity = 'error' | 'warning';

export interface Problem {
	/** A spore's or a thread's id. */
	at: string;
	severity: Severity;
	text: string;
}

export interface Group {
	id: string;
	label: string;
	/** `#rrggbb`. */
	colour?: string;
	/** The spores it frames – the one place a spore's group is kept. */
	spores: string[];
	folded?: boolean;
}

export interface Note {
	id: string;
	text: string;
	/** Who wrote it (the member's name). */
	author?: string;
	/** The spore it sits beside; none: after the weave. */
	near?: string;
}

export interface Point {
	x: number;
	y: number;
}

/** Authoring data beside the documents, never played – the shape deck-show keeps per row (deck/show/src/canvas.rs): groups, notes, spores folded (true) or opened (false) by hand over how they come, positions placed by hand (rem) over the auto layout. */
export interface Canvas {
	groups?: Group[];
	notes?: Note[];
	/** By spore id: folded (true) or opened (false) by hand; none: as it comes. */
	folded?: Record<string, boolean>;
	placed?: Record<string, Point>;
}

/** One level of the spore network: its spores, the threads woven between them, the threads that leave it. */
export interface Weave {
	scope: Scope;
	/** The level's address. */
	context: string;
	levels: Level[];
	spores: Spore[];
	threads: Thread[];
	stubs: Stub[];
	problems: Problem[];
	canvas?: Canvas;
}
