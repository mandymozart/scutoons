// Types of room-stage.js (the lab's 3D room – agents/research/LOCATION-SCENES.md, issue #74)
export interface RoomThing {
	id: string;
	/** the room's spot (`spot_<id>` node in the .glb) */
	spot: string;
	kind: 'pickup' | 'resident';
	/** a .glb (an item's or a persona's model) – else `icon` stands as a card */
	model?: string;
	icon?: string;
	/** metres; a model is scaled to it, an icon card is this high */
	height?: number;
}

export interface RoomStats {
	ms: { three: number; fetch: number; parse: number; things: number };
	mb: number;
	tris: number;
	calls: number;
	textures: number;
	pixelRatio: number;
	splats: number;
	fps: number;
}

export interface RoomStage {
	debug: Record<string, unknown>;
	take(id: string): void;
	allowTilt(): Promise<void>;
	dispose(): void;
}

export function mountRoom(
	canvas: HTMLCanvasElement,
	opts: {
		url: string;
		things: RoomThing[];
		maxPixelRatio?: number;
		maxZoom?: number;
		/** overrides the room's own view / spots (a `.glb`'s nodes, a `.splat`'s `.splat.json`) */
		view?: { at: number[]; look: number[]; fov?: number };
		spots?: { id: string; at: number[] }[];
		onstats?: (s: RoomStats) => void;
		ontap?: (hit: { thing?: string; point: number[] }) => void;
	}
): Promise<RoomStage>;
