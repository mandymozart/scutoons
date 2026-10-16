// Types of record-stage.js (plain JS: three.js has no bundled types and the repo carries no @types/three).

export interface RecordStageState {
	/** a song plays: the record comes out of the sleeve onto the platter and turns at 33⅓ rpm */
	playing: boolean;
	/** the side that lies up on the platter */
	side: 'a' | 'b';
	/** record-deck's tonearm angle in degrees (`armAngle`) */
	arm: number;
	/** between two songs: the arm is lifted */
	lifted: boolean;
	/** the key of the record that plays (`StageRecord.key`) – on play it is put on the platter */
	current: string | null;
}

export interface StageRecord {
	key: string;
	/** a .glb's URL or a builder (THREE → the model; the shared record model) */
	source: string | ((THREE: any) => Promise<any>);
}

export interface RecordStageHandle {
	update(state: Partial<RecordStageState>): void;
	/** stops the stage and frees the GPU memory */
	dispose(): void;
	/** eases the reader's look-around (pivot / zoom) back to the stage's own view */
	resetView(): void;
	/** the point the camera looks at and rides around (the focused part's centre), the reader's ride and zoom */
	inspect(): { target: [number, number, number]; yaw: number; zoom: number; tilt: [number, number]; arm: { x: number; y: number; turn: number; rest: number; outer: number; inner: number } | null };
}

/** the reader's records as a carousel (each sleeve + record) and a turntable on a canvas */
export function mountRecordStage(
	canvas: HTMLCanvasElement,
	records: StageRecord[],
	opts?: {
		onload?: () => void;
		onerror?: (error: unknown) => void;
		/** the reader set the tonearm down on the record (play from `progress` 0…1 of the song) or back at rest (pause) */
		onarm?: (to: { play: true; progress: number } | { play: false }) => void;
		/** a record was carried onto the turntable: put it on (its album plays) */
		onplay?: (key: string) => void;
		/** a picture for the table's surface (URL); none = plain brown */
		table?: string | null;
		/** the front record of the carousel changed (a swipe, or the stage turned to the one it puts on) */
		onselect?: (key: string) => void;
		/** 0…1 of the height at the bottom kept for controls laid over the canvas: the scene moves up */
		lift?: number;
		/** the record is already on (playing / paused in the album): it starts on the platter, no way out of the sleeve */
		placed?: boolean;
	}
): RecordStageHandle;
