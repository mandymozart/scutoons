// controls.ts (#209, #369): the browser's bridge to the deck's controls. The logic is deck-space's (its controls, as wasm:
// space-web's `DeckControls` – DECISIONS 2026-10-31 "the Svelte /live wrapper is only the bridge"): what the selection
// gives the two corner sticks, the sticks' relative moves, MIDI learn, the mappings' text, a knob's value ↔ 0..1, its
// turn and key steps, the panel's line. Here: the deck's values in and out (`setParam`), the knobs handed on as the core
// answered them (`DeckState.sliders`), the mappings per deck in localStorage until a preset (#191) carries them.
import type { Deck } from './deck.ts';
import type { Spec } from './scene.ts';

/** the wasm-bindgen API of deck/space-web's `DeckControls` */
export interface DeckControlsView {
	/** another deck's knobs (`DeckState.sliders`' JSON) and the being's stick pairs (x, y left, x, y right; fewer than four = the defaults) */
	set_deck(sliders: string, sticks: string[]): void;
	select(name: string): void;
	selection(): string;
	/** the knob that owns the sticks ('' none) */
	knob(): string;
	stick_paths(side: number): string[];
	stick_labels(side: number): string[];
	/** what the sticks drive, in one line (the words for the being and the master section: the page's language) */
	selection_line(being: string, master: string): string;
	stick_down(side: number, x: number, y: number): void;
	stick_move(side: number, dx: number, dy: number): Float32Array | number[];
	stick_driving(side: number): string[];
	stick_up(side: number): void;
	assign(side: number, axis: number, path: string): void;
	/** a knob's value ↔ 0..1 on its range (NaN: its default; undefined: no such knob); a drag by `dy` (screen y) from its
	 *  value `start` (`travel` turns it end to end, `fine` a tenth as fast); a key (the DOM's `key`; undefined: none) */
	norm(path: string, value: number): number;
	value_at(path: string, t: number): number | undefined;
	turn(path: string, start: number, dy: number, travel: number, fine: boolean): number | undefined;
	step(path: string, value: number, key: string, fine: boolean): number | undefined;
	/** one gamepad poll (the standard mapping; empty = no pad) → `[kind, side, dx, dy]` per event (gamepad.ts) */
	pad(axes: Float32Array | number[], buttons: Uint8Array | number[]): Float32Array | number[];
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

/** what a knob on the page needs of the controls */
export type KnobTurns = Pick<DeckControlsView, 'norm' | 'turn' | 'step'>;

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

/** A thumb on a stick: from where it landed to where it is, in stick radii, y up (the core keeps it inside the circle). */
export function deflection(from: [number, number], to: [number, number], radius: number): [number, number] {
	const r = radius || 1;
	return [(to[0] - from[0]) / r, -(to[1] - from[1]) / r];
}

/** the 270° arc of a knob at 0..1 – the SVG path's end point on a circle of radius `r` around (c, c), from 7:30 o'clock */
export function arcPoint(t: number, r: number, c: number): [number, number] {
	// core-only-ok: #369 drawing: the arc end of a position the core gave
	const a = ((135 + 270 * t) * Math.PI) / 180;
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
		specs: Record<string, Spec>,
		private readonly store: Store = localStore,
		private key = ''
	) {
		this.useParams(specs, null);
	}

	/** the deck's mappings from the store (its character's key) */
	load(character: string) {
		this.key = mappingKey(character);
		this.problems = this.c.load_text(this.store.get(this.key) ?? '');
	}

	private save() {
		if (this.key) this.store.set(this.key, this.c.to_text());
	}

	/** where knob `path`'s value sits, 0..1 (the core's) */
	norm(path: string): number {
		return this.c.norm(path, this.deck.value[path] ?? NaN);
	}

	/** knob `path` to `norm` (0..1) */
	set(path: string, norm: number) {
		const v = this.c.value_at(path, norm);
		if (v !== undefined) this.deck.setParam(path, v);
	}

	/** The deck's knobs changed (a rig's, or none again, #191, #360): the controls follow, with the being's
	 *  stick pairs (null = the defaults), the knobs as the core answered them. The selection goes back to the master section. */
	useParams(specs: Record<string, Spec>, sticks: [string, string][] | null) {
		this.c.set_deck(JSON.stringify(Object.entries(specs).map(([path, s]) => ({ ...s, path }))), sticks ? sticks.flat() : []);
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
