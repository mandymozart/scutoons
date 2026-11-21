// GENERATED from server/schema/api/network-view.schema.json by server/schema/generate.ts – edit the source, run `npm run generate`.
// One level of the one network (#763, agents/research/redesign/NETWORK.md §4): what a projector computes from a scope's documents and every canvas draws – the creator's Flow tab, the native deck editor, the Reception tab, the MCP. The documents stay the truth; a view is never stored (only its canvas). Each scope's projector lives where that scope's rules live (story: shared/content; XR: the OSCT logic package; show and rig: the Rust core). Edit this file, run `npm run generate` (server/schema/generate.ts writes the Rust and TS types), commit the outputs; network-view.vectors.json holds the views every language reads.
/** What travels on a port or edge – deck-kit's one signal table (deck/kit/src/base/signal.rs, owner 2026-11-17 [design]): its shapes and colours draw every port and wire. OSCT's `value` is `control`, its `node` is `address`, its `media` is `picture`. */
export type Signal = 'event' | 'control' | 'clock' | 'audio' | 'picture' | 'address' | 'vote' | 'scene' | 'far';

/** Which documents a view projects: `story` chapters and toons, `xr` experiences, `show` rows and cells, `rig` rigs and instruments, `scene` deck scenes, `creator` the creator's own events (generation done, approved, published …), `network` the levels above them all. */
export type Scope = 'story' | 'xr' | 'show' | 'rig' | 'scene' | 'creator' | 'network';

/** Where a node runs: the `reader` (the VN runner), `xr` (OSCT's engine in the reader), the `conductor`, a `deck` (the engine, its own audio context), the `server` (jobs, generations), the `editor` (authoring-time nodes only, never at reading time). */
export type Runtime = 'reader' | 'xr' | 'conductor' | 'deck' | 'server' | 'editor';

/** A node's family – how a canvas draws it and what a gesture may do. `container` can be entered (a chapter, a group, a row, a cell, a track); `world` comes from outside the level with an image of its origin (MIDI, clock, carrier bag, VN state); `free` lives only in the canvas (a reference, a generation, a manual instruction – OSCT L11–L13); `far` is a stub's far end. */
export type Family = 'event' | 'rule' | 'action' | 'variable' | 'entity' | 'scene' | 'plugin' | 'container' | 'sound' | 'world' | 'free' | 'far';

export type Direction = 'in' | 'out';

/** How an edge crosses runtimes: `same` (one runtime), `relay` (the Live relay), `navigation` (the reader moves to another place), `savegame` (a variable or item read elsewhere). */
export type Transport = 'same' | 'relay' | 'navigation' | 'savegame';

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
}

export interface Node {
	/** Unique within the view. */
	id: string;
	/** The registered node type (a scope's projector or a module names it). */
	kind: string;
	family: Family;
	label: string;
	detail?: string;
	/** What the node is, in the one grammar. */
	address: string;
	runtime: Runtime;
	ports: Port[];
	/** The address of the level a double-click opens; none: a leaf. */
	enter?: string;
	/** An asset reference for the node's image (a world node's origin, a preview). */
	picture?: string;
	/** The canvas group it sits in. */
	group?: string;
}

export interface Pin {
	node: string;
	port: string;
}

/** The one document field that holds an edge or a node: an edit is an op on it. None on an edge: derived (a by-name join, a computed link). */
export interface Home {
	/** The document's address, e.g. `rig:crow`, `chapter:sandbox/1/2`, `show:demo`. */
	doc: string;
	/** A JSON pointer into the document. */
	path: string;
}

export interface Edge {
	id: string;
	from: Pin;
	to: Pin;
	signal: Signal;
	label?: string;
	home?: Home;
	crosses?: Transport;
	/** A bundle of this many edges of one kind between one pair. */
	count?: number;
}

/** An edge that leaves the level, named by its far end; following it changes the view's scope. */
export interface Stub {
	/** The edge's id; its node at the level's edge is a `far` node. */
	edge: string;
	side: Direction;
	/** The far end's address. */
	far: string;
	label: string;
}

export type Severity = 'error' | 'warning';

export interface Problem {
	/** A node's or an edge's id. */
	at: string;
	severity: Severity;
	text: string;
}

export interface Group {
	id: string;
	label: string;
	colour?: string;
	nodes: string[];
	folded?: boolean;
}

export interface Note {
	id: string;
	text: string;
	/** The node it sits beside. */
	near?: string;
}

export interface Point {
	x: number;
	y: number;
}

/** Authoring data beside the documents, never played: groups, notes, folds, positions pinned by hand (rem) over the auto layout. */
export interface Canvas {
	groups?: Group[];
	notes?: Note[];
	folded?: string[];
	pins?: Record<string, Point>;
}

export interface View {
	scope: Scope;
	/** The level's address. */
	context: string;
	levels: Level[];
	nodes: Node[];
	edges: Edge[];
	stubs: Stub[];
	problems: Problem[];
	canvas?: Canvas;
}
