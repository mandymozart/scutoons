// Types of record-gestures.js (plain JS like record-stage.js).

/** what is under a point of the stage */
export interface StageHit {
	part: 'records' | 'deck';
	/** the record (its key) under the point, if any */
	key?: string | null;
	/** it is the carousel's front record */
	front?: boolean;
}

/** the stage's gestures (the owner's gesture map); returns a function that removes the listeners */
export function stageGestures(
	canvas: HTMLCanvasElement,
	on: {
		region: (x: number, y: number) => StageHit;
		/** the point lies in the stage's own part (above the card's text) */
		inStage: (x: number, y: number) => boolean;
		tap: (hit: StageHit) => void;
		swipe: (step: 1 | -1) => void;
		/** dx in canvas widths */
		ride: (dx: number) => void;
		/** a change of the 0…1 zoom */
		zoom: (by: number) => void;
		hold: () => void;
		release: () => void;
		/** picks the front record up (true) or puts it back (false); false = it can't be carried now */
		carry: (on: boolean) => boolean;
		/** the carried record follows the finger (canvas px) */
		carryTo: (x: number, y: number) => void;
		/** let go of the carried record over a part of the stage */
		drop: (part: 'records' | 'deck') => void;
	}
): () => void;
