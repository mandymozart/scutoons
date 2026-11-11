// live-page.ts (#478): the Live page in the browser deck, drawn by the core – deck-ui's page on the HUD (space-web
// `SceneView::live_*`), as the native deck's F6. Here only the DOM: the outlet's one stats line that opens it (the core's
// words), the keys and taps passed on whole (the core says whether it took them), the choice kept on this device as the
// core's text.

/** #478: the Live page's part of space-web's `SceneView` */
export interface LiveView {
	live_toggle(): boolean;
	live_key(key: string): boolean;
	live_tap(x: number, y: number): boolean;
	live_line(): string;
	live_kept(): string;
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
		this.line.onclick = () => view.live_toggle();
		host.append(this.line);
		addEventListener('keydown', this.key);
		canvas.addEventListener('pointerdown', this.tap);
	}

	/** each frame: the line as the core says it, the choice kept when the core's text changed */
	frame() {
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
		this.canvas.removeEventListener('pointerdown', this.tap);
		this.line.remove();
	}

	private key = (e: KeyboardEvent) => {
		if (this.view.live_key(e.key)) e.preventDefault();
	};

	private tap = (e: PointerEvent) => {
		const r = this.canvas.getBoundingClientRect();
		const k = this.canvas.width / Math.max(1, r.width);
		if (this.view.live_tap((e.clientX - r.left) * k, (e.clientY - r.top) * k)) e.preventDefault();
	};
}
