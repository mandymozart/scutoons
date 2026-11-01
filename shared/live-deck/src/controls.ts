// controls.ts (#209): the browser's bridge to the deck's controls. The logic is deck-space's (deck/space/src/controls.rs,
// as wasm: space-web's `DeckControls` – DECISIONS 2026-10-31 "the Svelte /live wrapper is only the bridge"): what the
// selection gives the two corner sticks, the sticks' relative moves, MIDI learn, the mappings' text. Here: the values'
// ranges (scene.ts Spec → 0..1 and back), the deck (`setParam`), the mappings per deck in localStorage until a preset
// (#191) carries them, and the frame statistics of the collapsed bar.
import type { Deck } from './deck.ts';
import { toPos, toValue, type Spec } from './scene.ts';

/** the wasm-bindgen API of deck/space-web's `DeckControls` */
export interface DeckControlsView {
	/** another deck's knobs and the being's stick pairs (x, y left, x, y right; fewer than four = the defaults) */
	set_deck(paths: string[], sticks: string[]): void;
	select(name: string): void;
	selection(): string;
	stick_paths(side: number): string[];
	stick_down(side: number, x: number, y: number): void;
	stick_move(side: number, dx: number, dy: number): Float32Array | number[];
	stick_driving(side: number): string[];
	stick_up(side: number): void;
	assign(side: number, axis: number, path: string): void;
	learn(path: string): void;
	learning(): string;
	midi(msg: Uint8Array): string;
	value(): number;
	learned(): boolean;
	mapped(path: string): string;
	unmap(path: string): void;
	to_text(): string;
	load_text(text: string): string[];
	free?(): void;
}

export const LEFT = 0;
export const RIGHT = 1;

/** where a deck's mappings live until a preset carries them (#191): per deck = per being (its character) */
export const mappingKey = (character: string) => `scutoons.live-deck.midi.${character || 'guest'}`;

/** a small key-value store (localStorage, or a stand-in in tests and private mode) */
export interface Store {
	get(key: string): string | null;
	set(key: string, value: string): void;
}

/** localStorage, quiet where it is blocked (private mode, a sandbox) */
export const localStore: Store = {
	get: (k) => {
		try {
			return localStorage.getItem(k);
		} catch {
			return null;
		}
	},
	set: (k, v) => {
		try {
			localStorage.setItem(k, v);
		} catch {
			/* private mode */
		}
	}
};

/** A thumb's deflection on a stick: from where it landed to where it is, in stick radii, inside the unit circle, y up. */
export function deflection(from: [number, number], to: [number, number], radius: number): [number, number] {
	const dx = (to[0] - from[0]) / Math.max(1e-6, radius);
	const dy = -(to[1] - from[1]) / Math.max(1e-6, radius);
	const len = Math.hypot(dx, dy);
	return len > 1 ? [dx / len, dy / len] : [dx, dy];
}

/** A rotary knob dragged by `dy` pixels (up = more): `range` pixels turn it end to end, `fine` (Shift) a tenth as fast. */
export function knobTurn(start: number, dy: number, range: number, fine = false): number {
	return Math.min(1, Math.max(0, start - (dy / Math.max(1, range)) * (fine ? 0.1 : 1)));
}

/** the 270° arc of a knob at 0..1 – the SVG path's end point on a circle of radius `r` around (c, c), from 7:30 o'clock */
export function arcPoint(t: number, r: number, c: number): [number, number] {
	const a = ((135 + 270 * Math.min(1, Math.max(0, t))) * Math.PI) / 180;
	return [c + r * Math.cos(a), c + r * Math.sin(a)];
}

/** One state's bridge: the controls (wasm) on the deck's values. */
export class Hands {
	/** what the controls said last (the panel's line): the selection and the sticks' paths */
	selection = '';
	/** lines of a stored mapping the deck couldn't use (shown once, never dropped silently) */
	problems: string[] = [];

	constructor(
		readonly c: DeckControlsView,
		private readonly deck: Pick<Deck, 'value' | 'setParam' | 'touch'>,
		private specs: Record<string, Spec>,
		private readonly store: Store = localStore,
		private key = ''
	) {}

	/** the deck's mappings from the store (its character's key) */
	load(character: string) {
		this.key = mappingKey(character);
		this.problems = this.c.load_text(this.store.get(this.key) ?? '');
	}

	private save() {
		if (this.key) this.store.set(this.key, this.c.to_text());
	}

	norm(path: string): number {
		const s = this.specs[path];
		return s ? Math.min(1, Math.max(0, toPos(s, this.deck.value[path] ?? s.value))) : 0;
	}

	set(path: string, norm: number) {
		const s = this.specs[path];
		if (s) this.deck.setParam(path, toValue(s, Math.min(1, Math.max(0, norm))));
	}

	/** The deck's knobs changed (a rig's, or the fixed patch's again, #191): the controls follow, with the being's
	 *  stick pairs (null = the fixed patch's defaults). The selection goes back to the master section. */
	useParams(specs: Record<string, Spec>, sticks: [string, string][] | null) {
		this.specs = specs;
		this.c.set_deck(Object.keys(specs), sticks ? sticks.flat() : []);
		this.selection = this.c.selection();
	}

	/** `""` master · `being` · `knob:<path>` */
	select(name: string) {
		this.c.select(name);
		this.selection = this.c.selection();
	}

	/** the [x, y] paths stick `side` drives now ("" none) */
	sticks(side: number): string[] {
		return [...this.c.stick_paths(side)];
	}

	stickDown(side: number) {
		const [x, y] = this.c.stick_paths(side);
		this.c.stick_down(side, this.norm(x), this.norm(y));
		this.deck.touch = 1;
	}

	/** the thumb's deflection (stick radii, y up) → the two values */
	stickMove(side: number, dx: number, dy: number) {
		const [nx, ny] = this.c.stick_move(side, dx, dy);
		const [px, py] = this.c.stick_driving(side);
		if (px) this.set(px, nx);
		if (py) this.set(py, ny);
	}

	stickUp(side: number) {
		this.c.stick_up(side);
		this.deck.touch = 0;
	}

	/** arm MIDI learn for `path` ("" disarms) */
	learn(path: string) {
		this.c.learn(path);
	}

	get learning(): string {
		return this.c.learning();
	}

	/** One MIDI message: the path it moved or learned (null = nothing) – a learned mapping is saved at once. */
	midi(msg: Uint8Array): { path: string; learned: boolean } | null {
		const path = this.c.midi(msg);
		if (!path) return null;
		if (this.c.learned()) {
			this.save();
			return { path, learned: true };
		}
		this.set(path, this.c.value());
		return { path, learned: false };
	}

	/** what drives `path` ("CC 7 · 1", "" none) */
	mapped(path: string): string {
		return this.c.mapped(path);
	}

	unmap(path: string) {
		this.c.unmap(path);
		this.save();
	}

	/** the mappings as a preset would carry them (#191) */
	get text(): string {
		return this.c.to_text();
	}
}

/** The collapsed bar's frame numbers: frames per second and the frame's own work (ms, the page's tick + the 3D's frame),
 *  over the last second. */
export class FrameStats {
	fps = 0;
	/** the average and the worst work per frame over the last second (ms) */
	cpu = 0;
	peak = 0;
	private start = -1;
	private frames = 0;
	private work = 0;
	private worst = 0;

	/** a frame at `now` (ms) that took `ms` of work */
	push(now: number, ms: number) {
		if (this.start < 0) {
			this.start = now;
			return;
		}
		this.frames++;
		this.work += ms;
		this.worst = Math.max(this.worst, ms);
		const span = now - this.start;
		if (span >= 1000) {
			this.fps = (this.frames * 1000) / span;
			this.cpu = this.work / this.frames;
			this.peak = this.worst;
			this.start = now;
			this.frames = this.work = this.worst = 0;
		}
	}
}
