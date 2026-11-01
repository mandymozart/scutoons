// The rig editor's form logic (#269): pure helpers the forms (instrument-form, preset-form, rig-form) call – what a
// select offers, how a preset's values read against its parent's folded ones, where a value is written. No DOM.
import { allInputs, audioOutputs, canonical, controlInputs, linksOf, nodeKinds, nodesOf, type LinkRef } from './graph.ts';
import { DEFAULT_RIGHTS, HOLDS, RIGHTS, fold, lineage, type Doc, type Lookup, type Resolved } from './validate.ts';

/** a lookup over a list of entries (`{ id, data }`) */
export const lookupOf = (entries: { id: string; data: Doc }[] | undefined): Lookup => {
	const map = new Map((entries ?? []).map((e) => [e.id, e.data]));
	return (id) => map.get(id) ?? null;
};

// ── Instrument ────────────────────────────────────────────────────────────────────────────────────────────

/** the `node.port` paths a preset may set, a knob may show (control inputs of the instrument's nodes) */
export const instrumentControls = (instrument: Doc | null | undefined): string[] => controlInputs(nodesOf(instrument?.node));
/** what `out` may name */
export const instrumentOutputs = (instrument: Doc | null | undefined): string[] => audioOutputs(nodesOf(instrument?.node));
/** every input, linked or not – what a preset may unlink or link into */
export const instrumentInputs = (instrument: Doc | null | undefined): string[] => allInputs(nodesOf(instrument?.node));

/** every port of the instrument's nodes as `node.port`: what a link may run from and into */
export function instrumentPorts(instrument: Doc | null | undefined): { inputs: string[]; outputs: string[] } {
	const KINDS = nodeKinds();
	const nodes = nodesOf(instrument?.node);
	return {
		inputs: nodes.flatMap((n) => Object.keys(KINDS[n.kind]?.inputs ?? {}).map((p) => `${n.id}.${p}`)),
		outputs: nodes.flatMap((n) => Object.keys(KINDS[n.kind]?.outputs ?? {}).map((p) => `${n.id}.${p}`))
	};
}

/** an exposed parameter's label in `locale`, else any, else its path */
export function exposeLabel(instrument: Doc | null | undefined, path: string, locale: string): string {
	const e = (Array.isArray(instrument?.expose) ? (instrument!.expose as Doc[]) : []).find((x) => canonical(String(x.path)) === canonical(path));
	const label = e?.label && typeof e.label === 'object' ? (e.label as Record<string, string>) : {};
	return label[locale] || label.en || Object.values(label).find(Boolean) || path;
}

// ── Preset ────────────────────────────────────────────────────────────────────────────────────────────────

/** the presets of one instrument */
export const presetsOf = (presets: { id: string; data: Doc }[] | undefined, instrument: string): { id: string; data: Doc }[] =>
	(presets ?? []).filter((p) => p.data?.instrument === instrument);

/** `id` and every preset below it (children, grandchildren …) – none of them may become its parent (a loop) */
export function descendants(presets: { id: string; data: Doc }[] | undefined, id: string): Set<string> {
	const out = new Set<string>([id]);
	let grew = true;
	while (grew) {
		grew = false;
		for (const p of presets ?? []) {
			if (!out.has(p.id) && typeof p.data?.parent === 'string' && out.has(p.data.parent)) {
				out.add(p.id);
				grew = true;
			}
		}
	}
	return out;
}

/** the presets `preset` may derive from: the same instrument, not itself nor one below it */
export function parentOptions(presets: { id: string; data: Doc }[] | undefined, preset: Doc): string[] {
	const below = descendants(presets, String(preset.id ?? ''));
	return presetsOf(presets, String(preset.instrument ?? ''))
		.map((p) => p.id)
		.filter((id) => !below.has(id));
}

/** the parent's lineage folded (what this preset inherits); null without a parent or when the lineage is broken */
export function inherited(preset: Doc, presets: Lookup): Resolved | null {
	if (typeof preset.parent !== 'string') return null;
	const chain = lineage(preset.parent, presets);
	return 'chain' in chain ? fold(chain.chain) : null;
}

export interface ValueRow {
	path: string;
	/** this preset's own value (what is stored) */
	own: number | undefined;
	/** the parent's folded value */
	inherits: number | undefined;
	/** the instrument's own param as written (a root's base) */
	base: number | undefined;
	/** the input has a link after the lineage's routing changes – a value here would be refused */
	linked: boolean;
	/** an exposed parameter (a knob) */
	exposed: boolean;
}

/** one row per control input of the instrument: the preset's value, what it inherits, whether the input is linked */
export function valueRows(instrument: Doc | null | undefined, preset: Doc, parent: Resolved | null): ValueRow[] {
	if (!instrument) return [];
	const own: Record<string, number> = {};
	for (const [k, v] of Object.entries(preset.values ?? {})) if (typeof v === 'number') own[canonical(k)] = v;
	const exposed = new Set((Array.isArray(instrument.expose) ? (instrument.expose as Doc[]) : []).map((e) => canonical(String(e.path))));
	const links = linkedInputs(instrument, fold([...(parent ? [] : []), preset]), parent);
	const nodes = nodesOf(instrument.node);
	return instrumentControls(instrument).map((path) => {
		const [node, port] = path.split('.');
		const raw = nodes.find((n) => n.id === node)?.params[port];
		return {
			path,
			own: own[path],
			inherits: parent?.values[path],
			base: typeof raw === 'number' ? raw : undefined,
			linked: links.has(path),
			exposed: exposed.has(path)
		};
	});
}

/** the inputs that have a link once the parent's and this preset's routing changes are applied */
export function linkedInputs(instrument: Doc, own: Resolved, parent: Resolved | null): Set<string> {
	const unlink = new Set([...(parent?.unlink ?? []), ...own.unlink]);
	const added: Record<string, LinkRef> = { ...(parent?.links ?? {}), ...own.links };
	for (const u of own.unlink) delete added[u];
	for (const to of Object.keys(own.links)) unlink.delete(to);
	const links = linksOf(instrument.link)
		.map((l) => canonical(l.to))
		.filter((to) => !unlink.has(to) && !(to in added));
	return new Set([...links, ...Object.keys(added)]);
}

/** the preset with `path` set to `value` – null removes it (the key goes; an empty map goes) */
export function withValue(preset: Doc, path: string, value: number | null): Doc {
	const values: Record<string, number> = { ...(preset.values ?? {}) };
	for (const k of Object.keys(values)) if (canonical(k) === canonical(path)) delete values[k];
	if (value !== null) values[path] = value;
	const { values: _old, ...rest } = preset;
	return Object.keys(values).length ? { ...rest, values } : rest;
}

// ── Rig ───────────────────────────────────────────────────────────────────────────────────────────────────

/** a free track id: `track1`, `track2` … */
export function newTrackId(rig: Doc): string {
	const taken = new Set((Array.isArray(rig.track) ? (rig.track as Doc[]) : []).map((t) => t.id));
	for (let n = 1; ; n++) if (!taken.has(`track${n}`)) return `track${n}`;
}

/** `<track>:<node>.<port>` for every control input of every track (a variant's single values, a route's end) */
export function trackPaths(rig: Doc, instruments: Lookup): string[] {
	return (Array.isArray(rig.track) ? (rig.track as Doc[]) : []).flatMap((t) => instrumentControls(instruments(String(t.instrument))).map((path) => `${t.id}:${path}`));
}

/** the space node's inputs a route may drive (`being:<input>`) – sound and image from one routing */
export const BEING_INPUTS = ['hue', 'glow', 'light', 'pulse'];

/** what a route may run from (`<track>:<node>.<port>` outputs) and into (inputs, and the being's look) */
export function routeEnds(rig: Doc, instruments: Lookup): { from: string[]; to: string[] } {
	const tracks = Array.isArray(rig.track) ? (rig.track as Doc[]) : [];
	const ports = tracks.map((t) => ({ id: String(t.id), ...instrumentPorts(instruments(String(t.instrument))) }));
	return {
		from: ports.flatMap((t) => t.outputs.map((p) => `${t.id}:${p}`)),
		to: [...ports.flatMap((t) => t.inputs.map((p) => `${t.id}:${p}`)), ...BEING_INPUTS.map((i) => `being:${i}`)]
	};
}

/** a variant (`personas.<p>` / `moods.<m>`) with `track`'s preset set – '' = as the track (the key goes) */
export function withVariantPreset(variant: Doc, track: string, preset: string): Doc {
	const presets: Record<string, string> = { ...(variant.presets ?? {}) };
	if (preset) presets[track] = preset;
	else delete presets[track];
	const { presets: _old, ...rest } = variant;
	return Object.keys(presets).length ? { ...rest, presets } : rest;
}

/** a variant with the single value `path` = `value`; null removes it */
export function withVariantValue(variant: Doc, path: string, value: number | null): Doc {
	const values: Record<string, number> = { ...(variant.values ?? {}) };
	if (value === null) delete values[path];
	else values[path] = value;
	const { values: _old, ...rest } = variant;
	return Object.keys(values).length ? { ...rest, values } : rest;
}

/** a variant `key` renamed in the map (`personas` / `moods`), keeping the order */
export function renameKey<T>(map: Record<string, T>, from: string, to: string): Record<string, T> {
	if (from === to || !to || to in map) return map;
	return Object.fromEntries(Object.entries(map).map(([k, v]) => [k === from ? to : k, v]));
}

/** the sticks as four knob paths (x, y of the left, then the right stick), "" = none */
export function sticksOf(rig: Doc): [string, string, string, string] {
	const s = Array.isArray(rig.sticks) ? (rig.sticks as string[][]) : [];
	return [s[0]?.[0] ?? '', s[0]?.[1] ?? '', s[1]?.[0] ?? '', s[1]?.[1] ?? ''];
}

/** the rig with one stick axis set; all four empty = the field goes (the first knobs) */
export function withStick(rig: Doc, index: 0 | 1 | 2 | 3, path: string): Doc {
	const s = sticksOf(rig);
	s[index] = path;
	const { sticks: _old, ...rest } = rig;
	return s.every((x) => x === '') ? rest : { ...rest, sticks: [[s[0], s[1]], [s[2], s[3]]] };
}

// ── Rights ────────────────────────────────────────────────────────────────────────────────────────────────

export type Rights = Record<(typeof HOLDS)[number], string[]>;

/** the rights as stored, the defaults filled in (mod.rs Rights::default) */
export function rightsOf(data: Doc): Rights {
	const r = data.rights && typeof data.rights === 'object' ? (data.rights as Partial<Rights>) : {};
	return Object.fromEntries(HOLDS.map((h) => [h, Array.isArray(r[h]) ? [...r[h]!] : [...DEFAULT_RIGHTS[h]]])) as Rights;
}

/** the entry with `right` toggled for `hold`; back at the defaults the field goes */
export function withRight(data: Doc, hold: (typeof HOLDS)[number], right: (typeof RIGHTS)[number], on: boolean): Doc {
	const rights = rightsOf(data);
	rights[hold] = RIGHTS.filter((r) => (r === right ? on : rights[hold].includes(r)));
	const { rights: _old, ...rest } = data;
	const isDefault = HOLDS.every((h) => rights[h].join() === DEFAULT_RIGHTS[h].join());
	return isDefault ? rest : { ...rest, rights };
}
