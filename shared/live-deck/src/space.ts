// space.ts (#198): the deck's being in 3D – deck-space compiled to wasm (deck/space-web: the same code as the native
// stage) drawing into a canvas, WebGPU where the browser has an adapter, else WebGL2. One state with the page: every
// frame the deck's values go in (`set`), what the being's buttons turned comes back (`changes` → `deck.setParam(…,
// 'being')`). The set and the portraits come from scene-content and the CDN, decoded by the browser.
//
// The wasm and its glue are files the host serves (`base`: space_web.js + space_web_bg.wasm, written by
// `npm run build:wasm -w shared/live-deck`) and imports at run time – a build without them still runs the deck, the
// page says the 3D is missing.
import type { DeckControlsView } from './controls.ts';
import type { Deck } from './deck.ts';
import { BPM } from './deck.ts';
import { PARAMS } from './scene.ts';

/** the wasm-bindgen API of deck/space-web (its `SpaceView`) */
export interface SpaceView {
	backend(): string;
	adapter(): string;
	resize(w: number, h: number): void;
	set_being(deckGraph: string, values: string): void;
	set(path: string, value: number): boolean;
	set_base(path: string, value: number): boolean;
	set_strain(reductionDb: number, dt: number): void;
	set_mood(mood: string): void;
	set_beat(bpm: number, beat: number, perBar: number, running: boolean): void;
	changes(): string;
	frame(dt: number, time: number): void;
	give_set(glb: Uint8Array, images: unknown[]): void;
	portrait_requests(): unknown[];
	give_portrait(slot: number, width: number, height: number, rgba: Uint8Array): void;
	no_portrait(slot: number, why: string): void;
	pointer_move(x: number, y: number): void;
	pointer_down(): void;
	pointer_up(): void;
	pointer_leave(): void;
	touch(id: number, phase: number, x: number, y: number): void;
	zoom(steps: number): void;
	home(): void;
	cursor(): string;
	hud_line(): string;
	mood(): string;
	/** #209: the selection a tap made since the last call ("" deselected, "being", "knob:node.port"), else undefined */
	take_tap(): string | undefined;
	/** #209: select by name – the being rings it */
	select(name: string): void;
	free(): void;
}

interface SpaceModule {
	default: (o: { module_or_path: string | URL }) => Promise<unknown>;
	SpaceView: {
		create(canvas: HTMLCanvasElement, deckGraph: string, values: string, prefer: string): Promise<SpaceView>;
		encoded_images(glb: Uint8Array): unknown[];
	};
	DeckControls: new (paths: string[]) => DeckControlsView;
}

const modules = new Map<string, Promise<SpaceModule>>();

/** the wasm and its glue from `base`, loaded once per page */
function spaceModule(base: string): Promise<SpaceModule> {
	let m = modules.get(base);
	if (!m) {
		m = (async () => {
			const mod = (await import(/* @vite-ignore */ `${base}space_web.js`)) as SpaceModule;
			await mod.default({ module_or_path: `${base}space_web_bg.wasm` });
			return mod;
		})();
		modules.set(base, m);
		m.catch(() => modules.delete(base));
	}
	return m;
}

/** #209: the deck's controls (deck-space's, as wasm – no GPU needed: they work when the 3D can't) over `paths` */
export async function loadControls(base: string, paths: string[]): Promise<DeckControlsView> {
	const mod = await spaceModule(base);
	return new mod.DeckControls(paths);
}

export interface SpaceOptions {
	/** where the canvas goes (filled; the stage makes and replaces its own canvas) */
	host: HTMLElement;
	/** the URL of the folder with space_web.js and space_web_bg.wasm (ends with /) */
	base: string;
	deck: Deck;
	/** the library API and the CDN (ends with /) */
	api: string;
	cdn: string;
	/** the set and its quality (scene-content) */
	set?: string;
	quality?: 'low' | 'medium' | 'high';
	/** "auto" (WebGPU if there is an adapter, else WebGL2) or "webgl2" */
	prefer?: 'auto' | 'webgl2';
	/** device pixels per CSS pixel at most (a phone's 3× costs fill rate for little) */
	maxScale?: number;
	onStatus?: (s: string) => void;
}

/** an image's bytes → RGBA8, decoded by the browser */
export async function decodeImage(bytes: Uint8Array | Blob, type = 'image/webp'): Promise<{ width: number; height: number; rgba: Uint8Array }> {
	const blob = bytes instanceof Blob ? bytes : new Blob([bytes as BlobPart], { type });
	const bmp = await createImageBitmap(blob, { colorSpaceConversion: 'none', premultiplyAlpha: 'none' });
	const { width, height } = bmp;
	const canvas = typeof OffscreenCanvas !== 'undefined' ? new OffscreenCanvas(width, height) : Object.assign(document.createElement('canvas'), { width, height });
	const g = canvas.getContext('2d') as CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D | null;
	if (!g) throw new Error('no 2d canvas to decode an image');
	g.drawImage(bmp, 0, 0);
	bmp.close();
	const data = g.getImageData(0, 0, width, height).data;
	return { width, height, rgba: new Uint8Array(data.buffer, data.byteOffset, data.byteLength) };
}

/** the image type of a file's first bytes (the .glb's images: WebP or PNG) */
export const sniff = (b: Uint8Array) => (b[0] === 0x89 && b[1] === 0x50 ? 'image/png' : b[0] === 0xff && b[1] === 0xd8 ? 'image/jpeg' : 'image/webp');

/** a pointer's position in the canvas' device pixels */
function at(canvas: HTMLCanvasElement, e: { clientX: number; clientY: number }): [number, number] {
	const r = canvas.getBoundingClientRect();
	return [((e.clientX - r.left) * canvas.width) / Math.max(1, r.width), ((e.clientY - r.top) * canvas.height) / Math.max(1, r.height)];
}

export class SpaceStage {
	private canvas!: HTMLCanvasElement;
	private stopped = false;
	private last = -1;
	private readonly fingers = new Map<number, [number, number]>();
	/** two fingers: their distance when the pinch started (device px) */
	private pinch = 0;
	private readonly portraitsByChar = new Map<string, Promise<Record<string, { path: string }>>>();
	private readonly cleanup: (() => void)[] = [];

	private constructor(
		private readonly o: SpaceOptions,
		private readonly mod: SpaceModule,
		public view: SpaceView
	) {}

	/** Load the wasm, make the view (WebGPU, else WebGL2 on a fresh canvas), fetch the set, wire the input. */
	static async start(o: SpaceOptions): Promise<SpaceStage> {
		const say = (s: string) => o.onStatus?.(s);
		say('3D: loading');
		const mod = await spaceModule(o.base);
		const graph = JSON.stringify(o.deck.graph());
		const values = JSON.stringify(o.deck.value);
		let canvas = SpaceStage.canvas(o);
		let view: SpaceView;
		try {
			view = await mod.SpaceView.create(canvas, graph, values, o.prefer ?? 'auto');
		} catch (e) {
			if (o.prefer === 'webgl2') throw e;
			// a canvas that had a WebGPU context can't give WebGL2: a fresh one
			console.warn('space: WebGPU failed, WebGL2 instead', e);
			canvas.remove();
			canvas = SpaceStage.canvas(o);
			view = await mod.SpaceView.create(canvas, graph, values, 'webgl2');
		}
		const s = new SpaceStage(o, mod, view);
		s.canvas = canvas;
		s.bind();
		say(`3D ${view.backend()}`);
		void s.loadSet().catch((e) => say(`3D: the set failed – ${(e as Error).message}`));
		return s;
	}

	private static canvas(o: SpaceOptions): HTMLCanvasElement {
		const c = document.createElement('canvas');
		c.style.cssText = 'display:block;width:100%;height:100%;touch-action:none;';
		o.host.append(c);
		const scale = Math.min(devicePixelRatio || 1, o.maxScale ?? 2);
		c.width = Math.max(1, Math.round(c.clientWidth * scale));
		c.height = Math.max(1, Math.round(c.clientHeight * scale));
		return c;
	}

	/** "webgpu" or "webgl2" */
	get backend(): string {
		return this.view.backend();
	}

	private async loadSet() {
		const set = this.o.set ?? 'cathedral';
		const quality = this.o.quality ?? 'low';
		const r = await fetch(`${this.o.api}/library/scene-content?sets=${encodeURIComponent(set)}`);
		const model = ((await r.json()) as { sets?: Record<string, { model?: Record<string, { path: string }> }> }).sets?.[set]?.model;
		const file = model?.[quality] ?? model?.low;
		if (!file) throw new Error(`no ${set} in scene-content`);
		this.o.onStatus?.(`3D: loading ${set}`);
		const glb = new Uint8Array(await (await fetch(this.o.cdn + file.path)).arrayBuffer());
		const encoded = this.mod.SpaceView.encoded_images(glb);
		const images: unknown[] = [];
		for (let i = 0; i + 1 < encoded.length; i += 2) {
			const bytes = encoded[i + 1] as Uint8Array;
			try {
				const img = await decodeImage(bytes, sniff(bytes));
				images.push(encoded[i], img.width, img.height, img.rgba);
			} catch (e) {
				console.warn(`space: image ${String(encoded[i])} not decoded`, e);
			}
		}
		if (this.stopped) return;
		this.view.give_set(glb, images);
		this.o.onStatus?.(`3D ${this.view.backend()}`);
	}

	private portraitsOf(character: string) {
		let p = this.portraitsByChar.get(character);
		if (!p) {
			p =
				character === this.o.deck.character && Object.keys(this.o.deck.portraits).length
					? Promise.resolve(this.o.deck.portraits)
					: fetch(`${this.o.api}/library/scene-content?characters=${encodeURIComponent(character)}`)
							.then((r) => r.json())
							.then((j: { characters?: Record<string, { portraits?: Record<string, { path: string }> }> }) => j.characters?.[character]?.portraits ?? {});
			this.portraitsByChar.set(character, p);
		}
		return p;
	}

	/** the sprites the heads asked for, fetched and decoded (each once) */
	private async portraits() {
		const asked = this.view.portrait_requests();
		for (let i = 0; i + 2 < asked.length; i += 3) {
			const [slot, character, mood] = [asked[i] as number, asked[i + 1] as string, asked[i + 2] as string];
			void (async () => {
				try {
					const p = (await this.portraitsOf(character))[mood];
					if (!p) throw new Error(`no ${character}_portrait-${mood}`);
					const img = await decodeImage(await (await fetch(this.o.cdn + p.path)).blob());
					if (!this.stopped) this.view.give_portrait(slot, img.width, img.height, img.rgba);
				} catch (e) {
					if (!this.stopped) this.view.no_portrait(slot, (e as Error).message);
				}
			})();
		}
	}

	/** the being after a character switch (the deck's graph and values now) */
	setBeing() {
		this.view.set_being(JSON.stringify(this.o.deck.graph()), JSON.stringify(this.o.deck.value));
	}

	/** One frame (after `deck.tick`): the being's turns into the deck, the deck's values into the being, draw. */
	frame(now: number) {
		if (this.stopped) return;
		const dt = this.last < 0 ? 1 / 60 : Math.min(0.1, now - this.last);
		this.last = now;
		const deck = this.o.deck;
		const turned = this.view.changes();
		if (turned) for (const [p, v] of Object.entries(JSON.parse(turned) as Record<string, number>)) deck.setParam(p, v, 'being');
		for (const p of deck.paths) {
			const v = deck.read(p);
			if (Number.isFinite(v)) this.view.set(p, v);
		}
		// #188: a linked knob (the incense cutoff the LFO sways) turns its base – the page's own value under the sway
		for (const p of Object.keys(PARAMS)) this.view.set_base(p, deck.value[p]);
		// the output limiter's gain reduction: the head's red flush
		this.view.set_strain(deck.bands.reduction ?? 0, dt);
		this.view.set_mood(deck.mood);
		const t = deck.time;
		this.view.set_beat(BPM, (t * BPM) / 60, 4, true);
		const c = this.canvas;
		const scale = Math.min(devicePixelRatio || 1, this.o.maxScale ?? 2);
		const w = Math.max(1, Math.round(c.clientWidth * scale));
		const h = Math.max(1, Math.round(c.clientHeight * scale));
		if (w !== c.width || h !== c.height) {
			c.width = w;
			c.height = h;
			this.view.resize(w, h);
		}
		this.view.frame(dt, t);
		const cursor = this.view.cursor();
		if (c.style.cursor !== cursor) c.style.cursor = cursor;
		void this.portraits();
	}

	/** pointer, touch and wheel → the stage's picking, drag, double click, orbit and zoom */
	private bind() {
		const c = this.canvas;
		const on = <K extends keyof HTMLElementEventMap>(type: K, f: (e: HTMLElementEventMap[K]) => void, opts?: AddEventListenerOptions) => {
			c.addEventListener(type, f as EventListener, opts);
			this.cleanup.push(() => c.removeEventListener(type, f as EventListener));
		};
		const pinchDistance = () => {
			const [a, b] = [...this.fingers.values()];
			return Math.hypot(a[0] - b[0], a[1] - b[1]);
		};
		on('pointerdown', (e) => {
			const [x, y] = at(c, e);
			c.setPointerCapture(e.pointerId);
			if (e.pointerType === 'touch') {
				this.fingers.set(e.pointerId, [x, y]);
				if (this.fingers.size === 1) this.view.touch(e.pointerId, 0, x, y);
				else if (this.fingers.size === 2) {
					// a second finger: the first lets go, the two pinch
					const [first] = this.fingers.keys();
					const p = this.fingers.get(first)!;
					this.view.touch(first, 2, p[0], p[1]);
					this.pinch = pinchDistance();
				}
			} else {
				this.view.pointer_move(x, y);
				this.view.pointer_down();
			}
			e.preventDefault();
		});
		on('pointermove', (e) => {
			const [x, y] = at(c, e);
			if (e.pointerType === 'touch') {
				if (!this.fingers.has(e.pointerId)) return;
				this.fingers.set(e.pointerId, [x, y]);
				if (this.fingers.size === 2 && this.pinch > 0) {
					const d = pinchDistance();
					// the camera's zoom: 12 % a step – the fingers' spread follows it
					this.view.zoom(Math.log(d / this.pinch) / -Math.log(0.88));
					this.pinch = d;
				} else if (this.fingers.size === 1 && this.pinch === 0) this.view.touch(e.pointerId, 1, x, y);
			} else this.view.pointer_move(x, y);
		});
		const up = (e: PointerEvent) => {
			const [x, y] = at(c, e);
			if (e.pointerType === 'touch') {
				const solo = this.fingers.size === 1 && this.pinch === 0;
				this.fingers.delete(e.pointerId);
				if (solo) this.view.touch(e.pointerId, 2, x, y);
				if (this.fingers.size === 0) this.pinch = 0;
			} else this.view.pointer_up();
		};
		on('pointerup', up);
		on('pointercancel', up);
		on('pointerleave', (e) => {
			if (e.pointerType !== 'touch') this.view.pointer_leave();
		});
		on(
			'wheel',
			(e) => {
				this.view.zoom(-e.deltaY / (e.deltaMode === 1 ? 3 : 100));
				e.preventDefault();
			},
			{ passive: false }
		);
		on('dblclick', (e) => e.preventDefault());
		on('contextmenu', (e) => e.preventDefault());
	}

	stop() {
		this.stopped = true;
		for (const f of this.cleanup) f();
		this.canvas.remove();
		this.view.free();
	}
}
