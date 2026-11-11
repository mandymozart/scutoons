// deck-panel.ts (#481): the taps and keys of the person at a browser deck, handed to the deck's own UI – the panel is
// drawn and decided in the core (deck-kit `panel.rs`, deck-show `flashed.rs` + `face.rs`); here only the events, their
// place on the canvas, and the input a field of the panel is typed in.

/** the scene view's side of the panel (space-web `SceneView`) */
export interface PanelView {
	panel_tap(x: number, y: number): boolean;
	panel_key(key: string): boolean;
	/** the text of the field being edited, when one is */
	panel_editing(): string | undefined;
	panel_field(text: string, cursor: number): void;
	set_may_reject(yes: boolean): void;
}

/** the keys that are the text's while the input has the focus */
const TEXT_KEYS = ['Backspace', 'Delete', 'ArrowLeft', 'ArrowRight', 'Home', 'End'];

/**
 * A tap on the canvas (in its pixels) and the page's keys go to the panel; what it took is not the page's any more.
 * A field of the panel is typed in a real input the page does not show: the browser's keyboard, an IME, paste and a
 * phone's keys work in it, and the panel draws what it holds. Returns how to stop.
 */
export function watchPanel(canvas: HTMLCanvasElement, view: PanelView): () => void {
	const input = document.createElement('input');
	input.setAttribute('aria-hidden', 'true');
	input.tabIndex = -1;
	// 16 px: a phone does not zoom in on a smaller input
	input.style.cssText = 'position:fixed;left:0;bottom:0;width:1px;height:1px;opacity:0;border:0;padding:0;font-size:16px';
	canvas.after(input);
	const mine = () => document.activeElement === input;
	const hand = () => view.panel_field(input.value, input.selectionStart ?? input.value.length);
	/** the input follows the core: it has the focus exactly while a field is edited */
	const follow = () => {
		const text = view.panel_editing();
		if (text === undefined) return input.blur();
		if (!mine()) {
			input.value = text;
			input.focus({ preventScroll: true });
		}
	};
	const tap = (e: PointerEvent) => {
		const r = canvas.getBoundingClientRect();
		if (view.panel_tap(((e.clientX - r.left) / r.width) * canvas.width, ((e.clientY - r.top) / r.height) * canvas.height)) e.preventDefault();
		follow();
	};
	const key = (e: KeyboardEvent) => {
		const typing = mine() && (e.key.length === 1 || TEXT_KEYS.includes(e.key));
		// a key typed into something else of the page (a field of a sheet) is that one's
		const elsewhere = !mine() && e.target instanceof HTMLElement && e.target.matches('input, textarea, select, [contenteditable]');
		if (!typing && !elsewhere && !e.ctrlKey && !e.metaKey && !e.altKey && view.panel_key(e.key)) e.preventDefault();
		follow();
	};
	canvas.addEventListener('pointerdown', tap);
	addEventListener('keydown', key);
	for (const what of ['input', 'keyup', 'select']) input.addEventListener(what, hand);
	return () => {
		canvas.removeEventListener('pointerdown', tap);
		removeEventListener('keydown', key);
		input.remove();
	};
}
