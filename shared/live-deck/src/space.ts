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
import type { DeckPushView } from './sync.ts';
import type { DeckQuality, QualityLevel } from '@scutoons/live';
import { nowMs, type MonitorView } from '@scutoons/live-core';
export type { QualityLevel };
export type { MonitorItem, MonitorReport, MonitorView } from '@scutoons/live-core';

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
	/** #387: the clock frame's beat heard at `heardMs`, drawn by the core at the page's `nowMs` */
	set_beat(bpm: number, beat: number, heardMs: number, nowMs: number, perBar: number, running: boolean): void;
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
	pointers(id: number, phase: number, x: number, y: number): void;
	zoom(steps: number): void;
	home(): void;
	cursor(): string;
	hud_line(): string;
	mood(): string;
	/** #209: the selection a tap made since the last call ("" deselected, "being", "knob:node.port"), else undefined */
	take_tap(): string | undefined;
	/** #209: select by name – the being rings it */
	select(name: string): void;
	/** #220: one frame's interval for the governor (now s, dt ms unclamped, the audio's dropouts so far); true = it stepped */
	govern(now: number, dtMs: number, glitches: number): boolean;
	/** #220: JSON – see `SpaceQuality` */
	quality(): string;
	/** #220: the `deck.quality` to send now (JSON) or "" */
	quality_report(now: number): string;
	/** #220: the panel's level ("low" … "ultra") and whether the governor steps below it */
	set_quality(level: string, auto: boolean): void;
	/** #220: the preset's frame rate cap: draw now, this many ms after the last drawn frame? */
	due(sinceMs: number): boolean;
	/** #220: the presets – the stage's quality file (TOML) and the sheet's JSON; setters answer "" or why not */
	presets_text(): string;
	presets_json(): string;
	set_presets(text: string): string;
	set_presets_json(json: string): string;
	/** #375: the option lists the settings offer (JSON, see `QualityOptions`) – deck-space's, the stage's page reads the same */
	options_json(): string;
	/** #220: the presets' pixel ratio cap (0 = none) */
	pixel_ratio(): number;
	free(): void;
}

/** #367, #368: the wasm-bindgen API of deck/space-web's `DeckState` – deck-space's `deck::Deck` without a GPU: the mood's
 *  energy ladder and what plays (its own scene or a rig, replayed on every mood). JSON answers: rig.ts. */
export interface DeckStateView {
	/** the character's portraits (their moods): the ladder anew */
	set_portraits(moods: string[]): void;
	/** hold the mood; '' = the energy walks it again */
	pin(mood: string): void;
	/** one frame: the level 0..1, the clock's bpm and whether it runs, a hand 0..1, dt s, a code rig's `presets` outputs as
	 *  heard (#410, NaN = not heard) → a `MoodStep` when it turned or the policy chose another preset, else '' */
	step(level: number, bpm: number, running: boolean, touch: number, dt: number, heard: Float32Array): string;
	/** #410: the paths of a code rig's preset outputs (`policy.<out>`) the frame hands to `step` as heard, in order */
	preset_paths(): string[];
	mood(): string;
	energy(): number;
	/** the moods on the wire: the ladder, then the other portraits */
	moods(): string[];
	/** the moods a picker offers */
	choices(): string[];
	/** the portrait a mood shows ('' none) */
	face(mood: string): string;
	/** what plays now, a `RigTurn` to load whole */
	now(): string;
	/** a rig bundle (JSON) for a persona and the mood now ('' = the deck's own scene) → a `RigTurn`; throws on a bad bundle */
	play(bundle: string, persona: string): string;
	/** the core rig of a character or a subject, else the deck's own scene → a `RigTurn` (its note says which) */
	play_core(character: string, persona: string): string;
	/** the knobs as `RigSlider`s in a locale ('' = English) */
	sliders(locale: string): string;
	/** the being on the wire (`deck.graph`); the deck's own scene wears `character` */
	graph(character: string): string;
	/** #282: the played scene's wiring (a `RigWiring`) */
	wiring(): string;
	/** where a value of a path goes in the engine ('' = nowhere) */
	target(path: string): string;
	/** #369: where a value sits on knob `path`'s range, 0..1 on its curve (NaN: its default; no such knob: 0) */
	norm(path: string, value: number): number;
	/** #369: knob `path`'s value at 0..1 (undefined: no such knob) */
	value_at(path: string, t: number): number | undefined;
	/** #370: the clock picker's choices (deck-core `ClockMode::ALL`) */
	clock_modes(): string[];
}

/** #220: the render quality now (deck-space quality.rs) */
export interface SpaceQuality {
	level: QualityLevel;
	/** the level chosen: the governor steps at most up to it */
	ceiling: QualityLevel;
	auto: boolean;
	/** the 3D's render size × the display's */
	scale: number;
	/** ms, the mean frame interval of the last half second */
	frame: number;
	/** the rung's frame rate cap */
	fps: number;
	width: number;
	height: number;
}

/** #375: the option lists the settings offer (deck-space quality.rs `options_json`) – the core's, the stage's page reads the same */
export interface QualityOptions {
	levels: { id: QualityLevel; label: string }[];
	/** the frame rate caps */
	fps: number[];
	/** the render scales (× the display's pixels) */
	scales: number[];
	/** the render heights (px) */
	heights: number[];
	/** the device pixel ratio caps (0 = none) */
	pixel_ratios: number[];
	max_lights: number;
	max_side: number;
}

/** #220: a render size – a scale of the display (≤ 4) or a height in pixels (≥ 100) */
export type RenderSize = number;

/** #220: one preset's knobs (deck-space quality.rs `Preset`) */
export interface QualityPreset {
	on: boolean;
	/** the render size under load and its own */
	min: RenderSize;
	max: RenderSize;
	fxaa: boolean;
	bloom: boolean;
	/** how many beings light the room (0–8) */
	lights: number;
	/** the frame rate cap */
	fps: number;
}

/** #220: the deck's quality settings – the same as the stage's quality file */
export interface QualityPresets {
	level?: QualityLevel;
	auto: boolean;
	display?: string;
	/** device pixels per CSS pixel at most */
	pixel_ratio?: number;
	low: QualityPreset;
	medium: QualityPreset;
	high: QualityPreset;
	ultra: QualityPreset;
}

export interface SpaceModule {
	default: (o: { module_or_path: string | URL }) => Promise<unknown>;
	SpaceView: {
		create(canvas: HTMLCanvasElement, deckGraph: string, values: string, prefer: string): Promise<SpaceView>;
		encoded_images(glb: Uint8Array): unknown[];
	};
	DeckControls: new () => DeckControlsView;
	DeckMonitor: new () => MonitorView;
	DeckState: new () => DeckStateView;
	DeckPush: new (api: string, cdn: string) => DeckPushView;
	rig_coverage(request: string): string;
	rig_catalog_bundle(request: string): string;
	rig_face(bundle: string): string;
	rig_policy_template(request: string): string;
}

const modules = new Map<string, Promise<SpaceModule>>();

/** the wasm and its glue from `base`, loaded once per page (assign.ts reads its free functions) */
export function spaceModule(base: string): Promise<SpaceModule> {
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

/** #209: the deck's controls (deck-space's, as wasm – no GPU needed: they work when the 3D can't); the knobs come with `set_deck` */
export async function loadControls(base: string): Promise<DeckControlsView> {
	const mod = await spaceModule(base);
	return new mod.DeckControls();
}

/** #367, #368: the deck's state – deck-space's mood ladder and what plays, its own scene (the X1's cathedral, #360) at
 *  first (as wasm, no GPU: it works when the 3D can't) */
export async function loadDeckState(base: string): Promise<DeckStateView> {
	const mod = await spaceModule(base);
	return new mod.DeckState();
}

/** #371: deck-space's push machine – the answers to the conductor's pushes (as wasm, no GPU); the library API and the CDN root */
export async function loadPush(base: string, api: string, cdn: string): Promise<DeckPushView> {
	const mod = await spaceModule(base);
	return new mod.DeckPush(api, cdn);
}

/** #364: deck-core's monitor (as wasm, no GPU: it works when the 3D can't, and before the sound) */
export async function loadMonitor(base: string): Promise<MonitorView> {
	const mod = await spaceModule(base);
	return new mod.DeckMonitor();
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
// core-only-ok: #357 an image's magic bytes (PNG / JPEG), not MIDI
export const sniff = (b: Uint8Array) => (b[0] === 0x89 && b[1] === 0x50 ? 'image/png' : b[0] === 0xff && b[1] === 0xd8 ? 'image/jpeg' : 'image/webp');

/** a pointer's position in the canvas' pixels (the render size – the view reads them so) */
function at(canvas: HTMLCanvasElement, e: { clientX: number; clientY: number }): [number, number] {
	const r = canvas.getBoundingClientRect();
	return [((e.clientX - r.left) * canvas.width) / Math.max(1, r.width), ((e.clientY - r.top) * canvas.height) / Math.max(1, r.height)];
}

export class SpaceStage {
	private canvas!: HTMLCanvasElement;
	private stopped = false;
	private last = -1;
	/** #220: the display size given to the view (the canvas' own size is the view's render size) */
	private display: [number, number] = [0, 0];
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
		// #367, #368: the deck's state (its own scene, #360, the mood ladder) before the being is built from the deck's graph
		if (!o.deck.state) o.deck.useState(new mod.DeckState());
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
		s.display = [canvas.width, canvas.height];
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

	/** #220: the render quality now */
	get quality(): SpaceQuality {
		return JSON.parse(this.view.quality()) as SpaceQuality;
	}

	/** #220: the panel's choice – the level (the ceiling) and the governor on / off */
	setQuality(level: QualityLevel, auto: boolean) {
		this.view.set_quality(level, auto);
	}

	/** #220: the presets (the settings sheet) */
	get presets(): QualityPresets {
		return JSON.parse(this.view.presets_json()) as QualityPresets;
	}

	/** #220: new presets – "" or why not */
	setPresets(p: QualityPresets): string {
		return this.view.set_presets_json(JSON.stringify(p));
	}

	/** #375: the option lists the settings offer, from the core */
	get qualityOptions(): QualityOptions {
		return JSON.parse(this.view.options_json()) as QualityOptions;
	}

	/** #220: the presets as the stage's quality file */
	get presetsText(): string {
		return this.view.presets_text();
	}

	setPresetsText(text: string): string {
		return this.view.set_presets(text);
	}

	/** #220: the device pixels per CSS pixel the 3D's display has: the presets' cap, else the page's (`maxScale`) */
	private pixelRatio(): number {
		return Math.min(devicePixelRatio || 1, this.view.pixel_ratio() || this.o.maxScale || 2);
	}

	/** #220: the most this screen can show – its physical pixels – and the display the 3D scales from now */
	get screen(): { max: [number, number]; display: [number, number] } {
		const r = devicePixelRatio || 1;
		return { max: [Math.round(screen.width * r), Math.round(screen.height * r)], display: [...this.display] };
	}

	/** One frame (after `deck.tick`): the being's turns into the deck, the deck's values into the being, draw. Returns
	 *  whether it drew (#364: the monitor counts drawn frames). */
	frame(now: number): boolean {
		if (this.stopped) return false;
		// #220: the preset's frame rate cap – an early frame is skipped (no draw: the power the cap saves); the governor
		// sees the real interval between drawn frames (a stall is a hitch to it, not load) and the audio's dropouts
		if (this.last >= 0 && !this.view.due((now - this.last) * 1000)) return false;
		if (this.last >= 0) this.view.govern(now, (now - this.last) * 1000, this.o.deck.core?.stats().underruns ?? 0);
		const report = this.view.quality_report(now);
		if (report) this.o.deck.link?.quality(JSON.parse(report) as DeckQuality);
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
		for (const p of Object.keys(deck.params)) this.view.set_base(p, deck.value[p]);
		// the output limiter's gain reduction: the head's red flush
		this.view.set_strain(deck.bands.reduction ?? 0, dt);
		this.view.set_mood(deck.mood);
		const t = deck.time;
		// #321: the being breathes on the deck's clock (the show clock's); before the sound starts it keeps its own
		const clock = deck.clock;
		if (clock) this.view.set_beat(clock.bpm, clock.beat, clock.heardMs, nowMs(), clock.perBar, clock.running);
		const c = this.canvas;
		const scale = this.pixelRatio();
		const w = Math.max(1, Math.round(c.clientWidth * scale));
		const h = Math.max(1, Math.round(c.clientHeight * scale));
		if (w !== this.display[0] || h !== this.display[1]) {
			this.display = [w, h];
			this.view.resize(w, h);
		}
		this.view.frame(dt, t);
		const cursor = this.view.cursor();
		if (c.style.cursor !== cursor) c.style.cursor = cursor;
		void this.portraits();
		return true;
	}

	/** pointer, touch and wheel → the stage's picking, drag, double click, orbit and zoom */
	private bind() {
		const c = this.canvas;
		const on = <K extends keyof HTMLElementEventMap>(type: K, f: (e: HTMLElementEventMap[K]) => void, opts?: AddEventListenerOptions) => {
			c.addEventListener(type, f as EventListener, opts);
			this.cleanup.push(() => c.removeEventListener(type, f as EventListener));
		};
		// a finger goes to the stage whole (id, phase 0 down · 1 moved · 2 up, place): which finger drives, the pinch and its zoom are deck-space's
		const finger = (e: PointerEvent, phase: number) => {
			const [x, y] = at(c, e);
			this.view.pointers(e.pointerId, phase, x, y);
		};
		on('pointerdown', (e) => {
			c.setPointerCapture(e.pointerId);
			if (e.pointerType === 'touch') finger(e, 0);
			else {
				const [x, y] = at(c, e);
				this.view.pointer_move(x, y);
				this.view.pointer_down();
			}
			e.preventDefault();
		});
		on('pointermove', (e) => {
			if (e.pointerType === 'touch') finger(e, 1);
			else this.view.pointer_move(...at(c, e));
		});
		const up = (e: PointerEvent) => {
			if (e.pointerType === 'touch') finger(e, 2);
			else this.view.pointer_up();
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
