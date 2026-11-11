// live-page.ts (#478, #386): the deck's menu and its Live page in the browser deck, drawn and decided by the core –
// deck-ui's menu and page (space-web `SceneView::menu_*`), as on the native deck. Here only the DOM: the outlet's one
// status line (the core's words) that opens the menu, the keys and the pointer passed on whole (the core says whether
// it took them), the page's place, the choice kept on this device as the core's text.

/** #478 #386: the menu's and the Live page's part of space-web's `SceneView` */
export interface LiveView {
	menu_open(): void;
	menu_key(key: string): boolean;
	/** press: 0 the pointer moved, 1 a press, 2 the secondary press */
	menu_pointer(x: number, y: number, press: number): boolean;
	live_line(): string;
	live_kept(): string;
	live_place(density: number, top: number): void;
}

/** where this device keeps the page's choice (private mode: nothing kept) */
const KEPT = 'scutoons.deck.live';

export function readKept(): string {
	try {
		return localStorage.getItem(KEPT) ?? '';
	} catch {
		return '';
	}
}

export class LivePage {
	private readonly line = document.createElement('button');

	constructor(
		private readonly view: LiveView,
		private readonly canvas: HTMLCanvasElement,
		host: HTMLElement,
		private kept: string
	) {
		this.line.type = 'button';
		this.line.dataset.testid = 'deck-live';
		this.line.style.cssText = 'position:absolute;top:0;left:0;right:0;border:0;text-align:left;font:0.625rem/1.5rem monospace;background:#000a;color:#fffd;cursor:pointer';
		this.line.onclick = () => view.menu_open();
		host.append(this.line);
		addEventListener('keydown', this.key);
		canvas.addEventListener('pointerdown', this.pointer);
		canvas.addEventListener('pointermove', this.pointer);
		canvas.addEventListener('contextmenu', this.pointer);
	}

	/** each frame: the page's place (the canvas pixels per CSS pixel, the line's height), the line as the core says it, the choice kept when the core's text changed */
	frame() {
		const k = this.canvas.width / (this.canvas.clientWidth || 1);
		this.view.live_place(k, this.line.offsetHeight * k);
		const text = this.view.live_line();
		if (this.line.textContent !== text) this.line.textContent = text;
		this.line.hidden = !text;
		const kept = this.view.live_kept();
		if (!kept || kept === this.kept) return;
		this.kept = kept;
		try {
			localStorage.setItem(KEPT, kept);
		} catch {
			/* private mode: not kept */
		}
	}

	stop() {
		removeEventListener('keydown', this.key);
		for (const what of ['pointerdown', 'pointermove', 'contextmenu'] as const) this.canvas.removeEventListener(what, this.pointer);
		this.line.remove();
	}

	private key = (e: KeyboardEvent) => {
		if (e.ctrlKey || e.metaKey || e.altKey) return;
		if (this.view.menu_key(e.key)) e.preventDefault();
	};

	private pointer = (e: PointerEvent | MouseEvent) => {
		// the secondary press comes as the context menu; a pointerdown of another button is nobody's here
		if (e.type === 'pointerdown' && e.button !== 0) return;
		const r = this.canvas.getBoundingClientRect();
		const k = this.canvas.width / Math.max(1, r.width);
		const press = e.type === 'pointermove' ? 0 : e.type === 'contextmenu' ? 2 : 1;
		if (this.view.menu_pointer((e.clientX - r.left) * k, (e.clientY - r.top) * k, press)) e.preventDefault();
	};
}
