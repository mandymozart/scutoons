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
}

export interface RecordStageHandle {
	update(state: Partial<RecordStageState>): void;
	/** stops the stage and frees the GPU memory */
	dispose(): void;
}

/** the LP's model (sleeve + record, `.set.glb`) and a turntable on a canvas */
export function mountRecordStage(canvas: HTMLCanvasElement, url: string, opts?: { onload?: () => void; onerror?: (error: unknown) => void }): RecordStageHandle;
