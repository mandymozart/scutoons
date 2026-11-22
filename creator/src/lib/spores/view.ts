// The Spores widget's adapter in the page (#763): loads spores-web (deck-spores in wasm, served from /live/ beside the
// deck's modules), sizes its canvas to the panel, runs its frames and hands it the pointer, the wheel and the keys. Every
// decision is the widget's; this only moves events in and the gestures out.

/** the wasm module's widget (deck/spores-web) */
interface SporesWidget {
	backend(): string;
	set_theme(night: boolean): void;
	resize(w: number, h: number, scale: number): void;
	set_weave(json: string): void;
	pointer_move(x: number, y: number): void;
	pointer_down(x: number, y: number, button: number, shift: boolean): void;
	pointer_up(): void;
	wheel(x: number, y: number, dx: number, dy: number, fingers: boolean, pinch: boolean): boolean;
	key(key: string, down: boolean, shift: boolean, ctrl: boolean): boolean;
	selection(): string;
	frame(secs: number): string;
	free(): void;
}

interface SporesModule {
	default(init: { module_or_path: string }): Promise<unknown>;
	Spores: { create(canvas: HTMLCanvasElement, prefer: string): Promise<SporesWidget> };
}

/** what the widget hands back: a gesture of deck-spores input.rs, as JSON */
export interface SporeGesture {
	kind: string;
	[k: string]: unknown;
}

const modules = new Map<string, Promise<SporesModule>>();

function sporesModule(base: string): Promise<SporesModule> {
	let m = modules.get(base);
	if (!m) {
		m = (async () => {
			const mod = (await import(/* @vite-ignore */ `${base}spores_web.js`)) as SporesModule;
			await mod.default({ module_or_path: `${base}spores_web_bg.wasm` });
			return mod;
		})();
		modules.set(base, m);
		m.catch(() => modules.delete(base));
	}
	return m;
}

/** The widget on `canvas` inside `host` (sized to it), until `stop`. */
export class SporesView {
	private widget: SporesWidget | null = null;
	private stopped = false;
	private raf = 0;
	private observer: ResizeObserver | null = null;
	private weave: string | null = null;
	private night = true;

	constructor(
		private host: HTMLElement,
		private canvas: HTMLCanvasElement,
		private ongesture: (g: SporeGesture) => void,
		private base = '/live/'
	) {}

	/** loads and starts; WebGPU where the browser has an adapter, else WebGL2. The backend's name. */
	async start(): Promise<string> {
		const mod = await sporesModule(this.base);
		let widget: SporesWidget;
		try {
			widget = await mod.Spores.create(this.canvas, 'auto');
		} catch {
			widget = await mod.Spores.create(this.canvas, 'webgl2');
		}
		if (this.stopped) {
			widget.free();
			return '';
		}
		this.widget = widget;
		widget.set_theme(this.night);
		if (this.weave) widget.set_weave(this.weave);
		this.observer = new ResizeObserver(() => this.fit());
		this.observer.observe(this.host);
		this.fit();
		this.bind();
		const frame = (now: number) => {
			if (this.stopped || !this.widget) return;
			const gestures = JSON.parse(this.widget.frame(now / 1000)) as SporeGesture[];
			for (const g of gestures) this.ongesture(g);
			this.raf = requestAnimationFrame(frame);
		};
		this.raf = requestAnimationFrame(frame);
		return widget.backend();
	}

	/** the weave to draw (the contract's JSON) */
	show(weave: object | null) {
		this.weave = weave ? JSON.stringify(weave) : null;
		if (this.widget && this.weave) this.widget.set_weave(this.weave);
	}

	theme(night: boolean) {
		this.night = night;
		this.widget?.set_theme(night);
	}

	/** what is selected, the focused spore first */
	selection(): string[] {
		return this.widget ? (JSON.parse(this.widget.selection()) as string[]) : [];
	}

	stop() {
		this.stopped = true;
		cancelAnimationFrame(this.raf);
		this.observer?.disconnect();
		this.unbind?.();
		this.widget?.free();
		this.widget = null;
	}

	private fit() {
		const k = Math.min(window.devicePixelRatio || 1, 2);
		const w = Math.max(1, Math.round(this.host.clientWidth * k));
		const h = Math.max(1, Math.round(this.host.clientHeight * k));
		this.canvas.style.width = `${this.host.clientWidth}px`;
		this.canvas.style.height = `${this.host.clientHeight}px`;
		this.widget?.resize(w, h, k);
	}

	private unbind: (() => void) | null = null;

	private bind() {
		const c = this.canvas;
		const at = (e: PointerEvent | WheelEvent) => {
			const r = c.getBoundingClientRect();
			return [e.clientX - r.left, e.clientY - r.top] as const;
		};
		const move = (e: PointerEvent) => this.widget?.pointer_move(...at(e));
		const down = (e: PointerEvent) => {
			if (e.button > 1) return;
			c.focus();
			c.setPointerCapture(e.pointerId);
			this.widget?.pointer_down(...at(e), e.button, e.shiftKey);
			e.preventDefault();
		};
		const up = (e: PointerEvent) => {
			if (c.hasPointerCapture(e.pointerId)) c.releasePointerCapture(e.pointerId);
			this.widget?.pointer_up();
		};
		const wheel = (e: WheelEvent) => {
			// a pinch comes as a wheel with ctrl; a mouse notch zooms, a trackpad's two fingers pan
			const pinch = e.ctrlKey;
			const fingers = e.deltaMode === 0 && !pinch && Math.abs(e.deltaY) < 50 && e.deltaX !== 0;
			const scale = e.deltaMode === 1 ? 1 : 1 / 100;
			if (this.widget?.wheel(...at(e), -e.deltaX * scale, -e.deltaY * scale, fingers, pinch)) e.preventDefault();
		};
		const key = (down: boolean) => (e: KeyboardEvent) => {
			if (this.widget?.key(e.key, down, e.shiftKey, e.ctrlKey || e.metaKey)) e.preventDefault();
		};
		const keydown = key(true);
		const keyup = key(false);
		c.tabIndex = 0;
		c.addEventListener('pointermove', move);
		c.addEventListener('pointerdown', down);
		c.addEventListener('pointerup', up);
		c.addEventListener('wheel', wheel, { passive: false });
		c.addEventListener('keydown', keydown);
		c.addEventListener('keyup', keyup);
		this.unbind = () => {
			c.removeEventListener('pointermove', move);
			c.removeEventListener('pointerdown', down);
			c.removeEventListener('pointerup', up);
			c.removeEventListener('wheel', wheel);
			c.removeEventListener('keydown', keydown);
			c.removeEventListener('keyup', keyup);
		};
	}
}
