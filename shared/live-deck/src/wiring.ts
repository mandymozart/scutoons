// wiring.ts (#282): the core's wiring of what a deck plays, as JSON types – no rule. The graph (nodes, their kinds,
// ports and depth, the links resolved to ports, the path of each cable's value) is deck-space's (rig/wiring.rs,
// space-web `DeckState.wiring`); the creator's Nodes tab only places and draws it.

export type WireType = 'audio' | 'control' | 'video';

export interface WirePort {
	name: string;
	type: WireType;
}

export interface WireNode {
	id: string;
	kind: string;
	/** the rig's track ('' = the scene's own: the sum, the outlet, the analyser, the being) */
	track: string;
	domain: WireType;
	/** 0 = fed by nothing before it, else one more than the deepest node feeding it */
	depth: number;
	inputs: WirePort[];
	outputs: WirePort[];
	/** the browser's engine plays it (false: no values on its cables here) */
	played: boolean;
}

export interface WireLink {
	/** indices into `nodes` and the ports' names */
	from: number;
	out: string;
	to: number;
	input: string;
	type: WireType;
	scale: number;
	offset: number;
	/** the path whose value flows on the cable (the output's `node.port`) */
	value: string;
	/** a module's path output bound to the input */
	bound?: boolean;
}

export interface RigWiring {
	nodes: WireNode[];
	links: WireLink[];
	/** why there is no graph */
	problem?: string;
}
