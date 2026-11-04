// The core's answers for the controls (#369): deck-space's controls – space-web's `DeckControls` – on the deck's own scene,
// recorded in test/fixtures/controls.json (written and kept current by `cargo test -p deck-space controls_fixture`): a
// pad script's events, a knob's value ↔ 0..1, its drags and key steps, the sticks' labels and the panel's line, a thumb's
// moves. The stand-in answers from it; a call the core never answered throws. No rule of the page's: the tests check
// that the bridge hands the core's answers on.
import { readFileSync } from 'node:fs';
import type { DeckControlsView } from '../src/controls.ts';
import type { RigSlider } from '../src/rig.ts';

interface Frame {
	axes: number[];
	buttons: boolean[];
	/** `[kind, side, dx, dy]` each */
	events: number[][];
	/** the selection after the frame */
	selection: string;
}
export const CONTROLS = JSON.parse(readFileSync(new URL('./fixtures/controls.json', import.meta.url), 'utf8')) as {
	knobs: RigSlider[];
	pad: Frame[];
	answers: Record<string, [unknown[], unknown][]>;
};

const table = new Map(Object.entries(CONTROLS.answers).map(([name, rows]) => [name, new Map(rows.map(([args, r]) => [JSON.stringify(args), r]))]));

/** the core's recorded answer to `name(...args)` (Rust's None: undefined) */
export function answer<T>(name: string, ...args: unknown[]): T {
	const key = JSON.stringify(args);
	const rows = table.get(name);
	if (!rows?.has(key)) throw new Error(`the core never answered ${name}(${key}) – UPDATE_FIXTURES=1 cargo test -p deck-space controls_fixture`);
	const r = rows.get(key);
	return (r === null ? undefined : r) as T;
}

/** A stand-in for space-web's `DeckControls` on the deck's own scene: the selection, the knobs and the sticks answer
 *  from the fixture, the pad replays its script frame by frame (and checks the page handed in the script's state); MIDI
 *  learn binds one CC (the mapping logic is tested in Rust); `calls` records what the page handed in. */
export function controlsOf(): DeckControlsView & { calls: unknown[][] } {
	const calls: unknown[][] = [];
	let selection = '';
	let frame = 0;
	let driving: string[] = ['', ''];
	let learning = '';
	let bound = '';
	let last = { value: 0, learned: false };
	return {
		calls,
		set_deck: (sliders, sticks) => void calls.push(['set_deck', JSON.parse(sliders), sticks]),
		select: (n) => void (selection = n),
		selection: () => selection,
		knob: () => answer('knob', selection),
		stick_paths: (side) => answer('stick_paths', selection, side),
		stick_labels: (side) => answer('stick_labels', selection, side),
		selection_line: (being, master) => answer('selection_line', selection, being, master),
		stick_down: (side, x, y) => {
			answer('stick_down', side, x, y);
			driving = answer('stick_paths', selection, side);
		},
		stick_move: (side, dx, dy) => answer('stick_move', side, dx, dy),
		stick_driving: () => driving,
		stick_up: (side) => void calls.push(['stick_up', side]),
		assign: () => {},
		norm: (path, value) => answer('norm', path, value),
		value_at: (path, t) => answer('value_at', path, t),
		turn: (path, start, dy, travel, fine) => answer('turn', path, start, dy, travel, fine),
		step: (path, value, key, fine) => answer('step', path, value, key, fine),
		pad: (axes, buttons) => {
			const f = CONTROLS.pad[frame++];
			calls.push(['pad', [...axes], [...buttons]]);
			selection = f.selection;
			return f.events.flat();
		},
		learn: (p) => void (learning = p),
		learning: () => learning,
		midi: (m) => {
			if (m[0] !== 0xb0) return '';
			if (learning) {
				[bound, learning, last] = [learning, '', { value: 0, learned: true }];
				return bound;
			}
			last = { value: m[2] / 127, learned: false };
			return bound;
		},
		value: () => last.value,
		learned: () => last.learned,
		mapped: (p) => (p === bound ? 'CC 7 · 1' : ''),
		unmap: () => void (bound = ''),
		to_text: () => (bound ? `cc 1 7 ${bound}\n` : ''),
		load_text: (t) => (t.includes('nowhere') ? ['cc 1 9 nowhere.x'] : [])
	};
}
