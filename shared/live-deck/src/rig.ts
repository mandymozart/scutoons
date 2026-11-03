// rig.ts (#191, research/INSTRUMENTS.md §7 step 3): the browser deck playing a character's rig instead of the fixed
// test patch (scene.ts). The rig is played in Rust (deck-space rig/, deck/space-web `rig_play` → `RigPlayed`); this
// module turns that answer into what the deck needs – its sliders (`Spec`s), the being on the wire, the paths it
// streams – and works out what a mood change alters (values only, or a new engine scene). Pure, no DOM, no wasm.
import type { DeckGraph } from '@scutoons/live';
import type { PathKind } from './client.ts';
import type { Spec } from './scene.ts';
import type { RigKnob, RigPlayed } from './space.ts';

/** a knob's label in `locale`, else English, else any, else its path */
export const knobLabel = (k: RigKnob, locale = 'en'): string =>
	k.label?.[locale] ?? k.label?.en ?? Object.values(k.label ?? {})[0] ?? k.path;

/** the deck's sliders: one per knob */
export function rigParams(p: RigPlayed, locale = 'en'): Record<string, Spec> {
	return Object.fromEntries(
		p.knobs.map((k) => [k.path, { min: k.min, max: k.max, value: k.value, label: knobLabel(k, locale), ...(k.log ? { log: true } : {}) }])
	);
}

/** the analyser's paths a rig's being shows as hoses (Rig::flatten always adds `fft` on the sum, mixers `mixN`) */
export function rigOutputs(p: RigPlayed): Record<string, PathKind> {
	const out: Record<string, PathKind> = {};
	if (p.nodes.some((n) => n.id === 'fft')) Object.assign(out, { 'fft.level': 'control', 'fft.bass': 'control' });
	const last = p.nodes.filter((n) => n.kind === 'mixer' && /^mix\d+$/.test(n.id)).pop();
	if (last) out[`${last.id}.out`] = 'audio';
	return out;
}

/** every path the deck streams: the knobs (inputs) + the analyser's outputs */
export function rigStreamed(p: RigPlayed): Record<string, PathKind> {
	return {
		...Object.fromEntries(p.knobs.map((k) => [k.path, { input: [k.min, k.max] as [number, number] }])),
		...rigOutputs(p)
	};
}

/** the being on the wire: the flattened scene's nodes and links (no params – the values stream), the knobs exposed */
export function rigGraph(p: RigPlayed, moods: string[]): DeckGraph {
	return {
		nodes: p.nodes.map((n) => ({ id: n.id, kind: n.kind })),
		links: p.links.map((l) => ({ from: l.from, to: l.to, ...(l.scale !== 1 ? { scale: l.scale } : {}), ...(l.offset ? { offset: l.offset } : {}) })),
		character: p.character,
		size: 0.5,
		moods,
		expose: p.knobs.map((k) => k.path)
	};
}

/** where a knob's value goes: live-core plays the rig's flattened scene as it is (#320), so a knob's path is the
 *  engine's – unless its node is one the browser does not carry yet (`engine.page`): then nowhere */
export function knobTarget(p: RigPlayed, path: string): { engine: string } | null {
	const node = path.slice(0, path.lastIndexOf('.'));
	if (p.engine.page.some((n) => n.id === node)) return null;
	return p.nodes.some((n) => n.id === node) ? { engine: path } : null;
}

/** The same scene for the engine? Values aside: the nodes (id, kind) and the links. A preset's routing change or
 *  another instrument needs live-core to load the new scene; otherwise only the changed values are sent. */
export function sameShape(a: RigPlayed, b: RigPlayed): boolean {
	const shape = (p: RigPlayed) => JSON.stringify([p.nodes.map((n) => [n.id, n.kind]), p.links.map((l) => [l.from, l.to, l.scale, l.offset])]);
	return shape(a) === shape(b);
}

/** every numeric param of the played nodes as `node.param` → value (what the preset and the variants wrote) */
function numericParams(p: RigPlayed): Map<string, number> {
	const out = new Map<string, number>();
	for (const n of p.nodes) for (const [k, v] of Object.entries(n)) if (typeof v === 'number') out.set(`${n.id}.${k}`, v);
	return out;
}

/** What a mood change alters: the values that moved – every knob and every numeric param the preset wrote, shown or
 *  not (path → new value) – and whether the engine reloads (another shape). */
export function rigDiff(from: RigPlayed, to: RigPlayed): { values: [string, number][]; reload: boolean } {
	const was = numericParams(from);
	for (const k of from.knobs) was.set(k.path, k.value);
	const now = numericParams(to);
	for (const k of to.knobs) now.set(k.path, k.value);
	const values = [...now].filter(([path, v]) => was.get(path) !== v);
	return { values, reload: !sameShape(from, to) };
}
