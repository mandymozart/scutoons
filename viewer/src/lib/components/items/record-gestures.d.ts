// Types of record-gestures.js (plain JS like record-stage.js).

export type StagePart = 'records' | 'deck';

/** the stage's gestures (the owner's gesture map); returns a function that removes the listeners */
export function stageGestures(
	canvas: HTMLCanvasElement,
	on: {
		region: (x: number, y: number) => StagePart;
		tap: (where: StagePart) => void;
		swipe: (step: 1 | -1) => void;
		/** dx in canvas widths */
		ride: (dx: number) => void;
		/** a change of the 0…1 zoom */
		zoom: (by: number) => void;
		hold: () => void;
		release: () => void;
	}
): () => void;
