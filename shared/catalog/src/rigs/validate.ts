// Instruments, presets and rigs checked in the creator before Publish (#269): what the server's RigValidator (and so
// deck-space's rig module) refuses – the same rules, worded the same, so a problem reads the same at the field and in
// the server's answer. Structure = schema.json (schema.ts), the rest mirrors field by field:
// - instrument.rs `Instrument::check`: a graph (graph.ts) without scene-only kinds, `out` an audio output, every exposed
//   path a control input, an events node's patch builds;
// - preset.rs `PresetBook::lineage` (an unknown parent, a loop, a parent of another instrument) and `Instrument::patch`
//   on the folded lineage (re-routed inputs exist, values on unlinked control inputs only, mapping targets, the graph);
// - mod.rs `Rig::check` (unique track ids without ':' / '.', known instruments, presets of the track's instrument,
//   variants and routes naming known tracks, `being:` only as a route's `to`) + the sticks (knobs of unmuted tracks).
import { RIG_SCHEMA, checkSchema, isNumber } from './schema.ts';
import { audioOutputs, buildGraph, canonical, checkEvents, controlInput, hasNull, kindsOf, linksOf, nodeKinds, nodesOf, split, type LinkRef } from './graph.ts';

export type Doc = Record<string, any>;
/** the catalog (and a member's own rows) by id; null / undefined = unknown */
export type Lookup = (id: string) => Doc | null | undefined;

export const INSTRUMENT_ID = /^[a-z0-9][a-z0-9_-]{0,63}$/;
/** a factory preset: dot-separated slugs (`cathedral_voice.bright`), ≤ 100 characters */
export const PRESET_ID = /^(?=.{1,100}$)[a-z0-9][a-z0-9_-]*(?:\.[a-z0-9][a-z0-9_-]*){0,3}$/;
/** a member's preset: `m:<ulid>` */
export const MEMBER_PRESET_ID = /^m:[0-9A-HJKMNP-TV-Z]{26}$/;
/** a rig's id = its subject (KANON's form): `character.<id>[.<persona>]` or `item.<id>` */
export const SUBJECT = /^(?:character\.[a-z0-9][a-z0-9_-]{0,63}(?:\.[a-z0-9][a-z0-9_-]{0,63})?|item\.[A-Za-z0-9][A-Za-z0-9_-]{0,63})$/;
/** instrument.rs InstrumentKind */
export const INSTRUMENT_KINDS = ['drum', 'bass', 'lead', 'pad', 'voice', 'sampler', 'texture', 'fx'] as const;
/** mod.rs Right */
export const RIGHTS = ['play', 'tweak', 'derive', 'copy'] as const;
/** mod.rs Hold */
export const HOLDS = ['lent', 'owned', 'core'] as const;
/** mod.rs Rights::default – what a definition without `rights` grants */
export const DEFAULT_RIGHTS: Record<(typeof HOLDS)[number], string[]> = { lent: ['play', 'tweak'], owned: ['play', 'tweak', 'derive', 'copy'], core: ['play', 'tweak', 'derive'] };

const isObj = (v: unknown): v is Doc => !!v && typeof v === 'object' && !Array.isArray(v);
const show = (v: unknown) => (v === null || v === undefined || typeof v === 'object' ? JSON.stringify(v) : String(v));

// ── Instrument ────────────────────────────────────────────────────────────────────────────────────────────

export function validateInstrument(i: unknown): string[] {
	if (!isObj(i)) return ['an instrument must be an object'];
	const id = typeof i.id === 'string' ? i.id : '?';
	const who = `instrument ${id}`;
	const p = checkSchema(RIG_SCHEMA.instrument, RIG_SCHEMA.instrument, i, who);
	if (typeof i.id === 'string' && !INSTRUMENT_ID.test(i.id)) p.push(`${who}: id must be lower-case letters, digits, _ or -`);
	if (p.length) return p;
	const nodes = nodesOf(i.node);
	for (const n of nodes) for (const [k, v] of Object.entries(n.params)) if (hasNull(v)) p.push(`${who}: node ${n.id}: ${k} has a null (TOML has none – leave it out)`);
	if (p.length) return p;
	const graph = buildGraph(who, nodes, linksOf(i.link));
	if ('problems' in graph) return graph.problems;
	for (const n of nodes) if (RIG_SCHEMA.scene_only.includes(n.kind)) p.push(`${who}: a ${n.kind} node belongs to the scene, not to an instrument (${n.id})`);
	const [node, port] = split(i.out);
	const kind = graph.kinds[node];
	if (kind === undefined || nodeKinds()[kind]?.outputs[port] !== 'audio') p.push(`${who}: out '${i.out}' is not an audio output`);
	for (const e of (i.expose ?? []) as Doc[]) {
		const problem = controlInput(who, graph.kinds, e.path);
		if (problem) p.push(problem);
	}
	for (const n of nodes) if (n.kind === 'events') p.push(...checkEvents(`${who}: events node '${n.id}'`, n.params));
	return p;
}

// ── Preset ────────────────────────────────────────────────────────────────────────────────────────────────

/**
 * A preset against its instrument and lineage; `presets` sees the catalog (and a member's own rows) – this entry
 * counts with its new data (the lookup is asked for every id but its own).
 */
export function validatePreset(pr: unknown, instruments: Lookup, presets: Lookup, idRule: RegExp = PRESET_ID): string[] {
	if (!isObj(pr)) return ['a preset must be an object'];
	const id = typeof pr.id === 'string' ? pr.id : '?';
	const who = `preset ${id}`;
	const p = checkSchema(RIG_SCHEMA.preset, RIG_SCHEMA.preset, pr, who);
	if (typeof pr.id !== 'string' || !idRule.test(pr.id)) p.push(`${who}: id is not a valid preset id`);
	for (const [k, v] of Object.entries(pr.values ?? {})) if (!isNumber(v)) p.push(`${who}: values.${k} must be a number`);
	for (const m of Array.isArray(pr.mapping) ? (pr.mapping as Doc[]) : []) if (m?.source === '') p.push(`${who}: mapping.source must be a source (midi:<ch>:cc:<n>, face:…, motion:…, pad:…, stick:…)`);
	if (typeof pr.instrument !== 'string') return [...p, `${who}: instrument must be an instrument id`];
	const instrument = instruments(pr.instrument);
	if (!instrument) return [...p, `${who}: unknown instrument "${pr.instrument}"`];
	if (p.length) return p;
	const lookup: Lookup = (pid) => (pid === id ? pr : presets(pid));
	const chain = lineage(id, lookup);
	if ('problem' in chain) return [chain.problem];
	return patch(who, instrument, chain.chain);
}

/**
 * `id` and its ancestors, root first (preset.rs `lineage`); the problem when it does not resolve: an unknown preset,
 * an unknown parent, a loop, a parent of another instrument
 */
export function lineage(id: string, presets: Lookup): { chain: Doc[] } | { problem: string } {
	const first = presets(id);
	if (!first) return { problem: `unknown preset "${id}"` };
	const chain = [first];
	const ids = [id];
	for (;;) {
		const child = chain[chain.length - 1];
		const parent = child.parent;
		if (typeof parent !== 'string') break;
		if (ids.includes(parent)) return { problem: `preset ${id}: its lineage loops at "${parent}"` };
		const next = presets(parent);
		if (!next) return { problem: `preset ${child.id}: unknown preset "${parent}" (its parent)` };
		if (next.instrument !== child.instrument) return { problem: `preset ${child.id} (${show(child.instrument)}) derives from "${parent}" of another instrument (${show(next.instrument)})` };
		chain.push(next);
		ids.push(parent);
	}
	return { chain: chain.reverse() };
}

/** A preset chain folded into what the instrument applies (preset.rs `Resolved`) */
export interface Resolved {
	instrument: string;
	values: Record<string, number>;
	unlink: Set<string>;
	/** added links by their input (`node.port`) */
	links: Record<string, LinkRef>;
	/** by `source\0target`: a child's mapping with the same pair replaces its parent's */
	mappings: Record<string, Doc>;
}

/** the chain folded root → leaf (preset.rs `Resolved::fold`) */
export function fold(chain: Doc[]): Resolved {
	const r: Resolved = { instrument: String(chain[0]?.instrument ?? ''), values: {}, unlink: new Set(), links: {}, mappings: {} };
	for (const pr of chain) {
		for (const [k, v] of Object.entries(pr.values ?? {})) if (typeof v === 'number') r.values[canonical(k)] = v;
		for (const u of (pr.unlink ?? []) as string[]) {
			const c = canonical(u);
			delete r.links[c];
			r.unlink.add(c);
		}
		for (const l of linksOf(pr.link)) {
			const to = canonical(l.to);
			r.unlink.delete(to);
			r.links[to] = l;
		}
		for (const m of Array.isArray(pr.mapping) ? (pr.mapping as Doc[]) : []) r.mappings[`${m.source}\0${canonical(String(m.target))}`] = m;
	}
	return r;
}

/**
 * instrument.rs `patch` on a folded chain: re-routed inputs exist, values only on unlinked control inputs, mapping
 * targets are control inputs, the patched graph builds
 */
function patch(who: string, instrument: Doc, chain: Doc[]): string[] {
	const p: string[] = [];
	if (validateInstrument(instrument).length) return [`${who}: its instrument is not valid`];
	const nodes = nodesOf(instrument.node);
	const kinds = kindsOf(nodes);
	const KINDS = nodeKinds();
	const r = fold(chain);
	for (const target of [...r.unlink, ...Object.keys(r.links)]) {
		const [node, port] = split(target);
		const kind = kinds[node];
		if (kind === undefined || !KINDS[kind]?.inputs[port]) p.push(`${who}: no input ${target} to re-route`);
	}
	if (p.length) return p;
	const links = [...linksOf(instrument.link).filter((l) => !r.unlink.has(canonical(l.to)) && !(canonical(l.to) in r.links)), ...Object.values(r.links)];
	for (const path of Object.keys(r.values)) {
		const problem = controlInput(who, kinds, path);
		if (problem) p.push(problem);
		else if (links.some((l) => canonical(l.to) === path)) p.push(`${who}: ${path} is linked – a preset value needs an unlinked input (unlink it first)`);
	}
	for (const m of Object.values(r.mappings)) {
		const problem = controlInput(who, kinds, canonical(String(m.target)));
		if (problem) p.push(problem);
	}
	if (p.length) return p;
	const graph = buildGraph(`${who} (with its instrument)`, nodes, links);
	return 'problems' in graph ? graph.problems : [];
}

// ── Rig ───────────────────────────────────────────────────────────────────────────────────────────────────

interface TrackInfo {
	instrument: string;
	def: Doc | null;
	mute: boolean;
}

/**
 * A rig (`id` = its subject) against the instruments and presets it names; `allowed` = the instruments it may use (a
 * member's rig: the ones they hold), null = any
 */
export function validateRig(r: unknown, id: string, instruments: Lookup, presets: Lookup, allowed: string[] | null = null): string[] {
	if (!isObj(r)) return ['a rig must be an object'];
	const who = `rig ${id}`;
	const p = checkSchema(RIG_SCHEMA.rig, RIG_SCHEMA.rig, r, who);
	if (!SUBJECT.test(id)) p.push(`${who}: the id is a subject – character.<id>[.<persona>] or item.<id>`);
	if (r.subject !== id) p.push(`${who}: subject must be "${id}"`);
	if (!Array.isArray(r.track)) return p;
	const tracks: Record<string, TrackInfo> = {};
	for (const t of r.track as Doc[]) {
		if (!isObj(t) || typeof t.id !== 'string' || typeof t.instrument !== 'string') continue;
		if (t.id in tracks) p.push(`${who}: track '${t.id}' twice`);
		if (t.id.includes(':') || t.id.includes('.')) p.push(`${who}: a track id has no ':' or '.' (${t.id})`);
		const instrument = instruments(t.instrument) ?? null;
		if (!instrument) p.push(`${who}: unknown instrument "${t.instrument}"`);
		else if (allowed && !allowed.includes(t.instrument)) p.push(`${who}: track ${t.id}: the instrument "${t.instrument}" is not one held`);
		tracks[t.id] = { instrument: t.instrument, def: instrument, mute: t.mute === true };
		if (typeof t.preset === 'string') p.push(...fits(who, t.id, t.instrument, t.preset, presets));
	}
	for (const [field, label] of [
		['personas', 'persona'],
		['moods', 'mood']
	] as const) {
		if (!isObj(r[field])) continue;
		for (const [name, v] of Object.entries(r[field] as Record<string, Doc>)) {
			const at = `${who}: ${label} ${name}`;
			if (!isObj(v)) continue;
			for (const [track, preset] of Object.entries(v.presets ?? {})) {
				if (!(track in tracks)) p.push(`${at}: no track '${track}'`);
				else if (typeof preset === 'string') p.push(...fits(at, track, tracks[track].instrument, preset, presets));
			}
			for (const [path, value] of Object.entries(v.values ?? {})) {
				if (!isNumber(value)) p.push(`${at}: ${path} must be a number`);
				const cut = path.indexOf(':');
				if (cut < 0) p.push(`${at}: '${path}' is not <track>:<node>.<port>`);
				else {
					const [track, inner] = [path.slice(0, cut), path.slice(cut + 1)];
					if (!(track in tracks)) p.push(`${at}: no track '${track}'`);
					else if (tracks[track].def) {
						const problem = controlInput(`instrument ${tracks[track].instrument}`, kindsOf(nodesOf(tracks[track].def!.node)), inner);
						if (problem) p.push(`${at}: ${problem}`);
					}
				}
			}
		}
	}
	if (r.sticks !== undefined && r.sticks !== null) p.push(...sticks(who, r.sticks, tracks));
	for (const route of Array.isArray(r.route) ? (r.route as Doc[]) : []) {
		if (!isObj(route)) continue;
		for (const end of ['from', 'to'] as const) {
			const ref = route[end];
			if (typeof ref !== 'string') continue;
			const cut = ref.indexOf(':');
			if (cut < 0) p.push(`${who}: route end '${ref}' has no scope`);
			else {
				const scope = ref.slice(0, cut);
				if (!(scope === 'being' && end === 'to') && !(scope in tracks)) p.push(`${who}: no track '${scope}' (route ${ref})`);
			}
		}
	}
	return p;
}

/** the knobs a rig shows: `<track>_<node>.<port>` for every exposed parameter of an unmuted track (mod.rs / bundle.rs) */
export function knobPaths(rig: Doc, instruments: Lookup): string[] {
	const out: string[] = [];
	for (const t of Array.isArray(rig.track) ? (rig.track as Doc[]) : []) {
		if (!isObj(t) || t.mute === true || typeof t.id !== 'string') continue;
		const def = instruments(String(t.instrument));
		for (const e of Array.isArray(def?.expose) ? (def!.expose as Doc[]) : []) if (typeof e?.path === 'string') out.push(`${t.id}_${canonical(e.path)}`);
	}
	return out;
}

/** mod.rs `sticks` + bundle.rs `play`: `[[x, y], [x, y]]` knob paths, each "" or a knob of an unmuted track */
function sticks(who: string, value: unknown, tracks: Record<string, TrackInfo>): string[] {
	const pair = (x: unknown) => Array.isArray(x) && x.length === 2 && typeof x[0] === 'string' && typeof x[1] === 'string';
	if (!(Array.isArray(value) && value.length === 2 && pair(value[0]) && pair(value[1]))) return [`${who}: sticks must be [[x, y], [x, y]] (knob paths, "" = none)`];
	const knobs: string[] = [];
	for (const [id, t] of Object.entries(tracks)) {
		if (t.mute || !t.def) continue;
		for (const e of Array.isArray(t.def.expose) ? (t.def.expose as Doc[]) : []) if (typeof e?.path === 'string') knobs.push(`${id}_${canonical(e.path)}`);
	}
	const p: string[] = [];
	for (const path of [...(value[0] as string[]), ...(value[1] as string[])]) if (path !== '' && !knobs.includes(path)) p.push(`${who}: stick '${path}' is not one of its knobs`);
	return p;
}

/** mod.rs `preset_fits`: a known preset of the track's instrument */
function fits(who: string, track: string, instrument: string, preset: string, presets: Lookup): string[] {
	const pr = presets(preset);
	if (!pr) return [`${who}: unknown preset "${preset}"`];
	if (pr.instrument !== instrument) return [`${who}: track '${track}' plays '${instrument}', preset '${preset}' is for '${show(pr.instrument)}'`];
	return [];
}

export { audioOutputs };
