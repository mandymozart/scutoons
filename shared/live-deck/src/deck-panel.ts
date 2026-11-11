// deck-panel.ts (#481): the taps and keys of the person at a browser deck, handed to the deck's own UI – the panel is
// drawn and decided in the core (deck-kit `panel.rs`, deck-show `flashed.rs`); here only the events and their place on
// the canvas.

/** the scene view's side of the panel (space-web `SceneView`) */
export interface PanelView {
	panel_tap(x: number, y: number): boolean;
	panel_key(key: string): boolean;
	set_may_reject(yes: boolean): void;
}

/** A tap on the canvas (in its pixels) and the page's keys go to the panel; what it took is not the page's any more. Returns how to stop. */
export function watchPanel(canvas: HTMLCanvasElement, view: PanelView): () => void {
	const tap = (e: PointerEvent) => {
		const r = canvas.getBoundingClientRect();
		if (view.panel_tap(((e.clientX - r.left) / r.width) * canvas.width, ((e.clientY - r.top) / r.height) * canvas.height)) e.preventDefault();
	};
	const key = (e: KeyboardEvent) => {
		if (view.panel_key(e.key)) e.preventDefault();
	};
	canvas.addEventListener('pointerdown', tap);
	addEventListener('keydown', key);
	return () => {
		canvas.removeEventListener('pointerdown', tap);
		removeEventListener('keydown', key);
	};
}
