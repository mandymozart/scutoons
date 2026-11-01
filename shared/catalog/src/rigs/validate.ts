// The creator's check of instruments, presets and rigs before the server's (#269, agents/CATALOG.md "Instruments,
// presets, rigs"): the serde forms from deck-space's generated schema.json (unknown fields, types, required fields),
// then what refers to what – deck-space's Rig::check, PresetBook::lineage and the node names an instrument's paths use.
// The node kinds' ports and types, cycles and an events node's patch stay with the server (RigValidator mirrors
// graph.rs): its 422 problems are shown at the fields. Messages read like the server's (`rig <id>: …`), so a form finds
// both by the same words (problemFor).
import SCHEMA from './schema.json' with { type: 'json' };
import { checkSchema, type SchemaPath, type SchemaRoot } from './schema-check.ts';

export type Data = Record<string, any>;
export type RigKind = 'instruments' | 'presets' | 'rigs';
export const RIG_KINDS: readonly RigKind[] = ['instruments', 'presets', 'rigs'];

export interface RigProblem {
	path: SchemaPath;
	message: string;
}

/** what a preset / rig refers to: the catalog's instruments and presets as they stand (null = none) */
export interface RigLibrary {
	instrument: (id: string) => Data | null;
	preset: (id: string) => Data | null;
}

/** RigValidator::INSTRUMENT_ID */
export const INSTRUMENT_ID = /^[a-z0-9][a-z0-9_-]{0,63}$/;
/** RigValidator::PRESET_ID: dot-separated slugs (`cathedral_voice.bright`), ≤ 100 characters */
export const PRESET_ID = /^(?=.{1,100}$)[a-z0-9][a-z0-9_-]*(?:\.[a-z0-9][a-z0-9_-]*){0,3}$/;
/** RigValidator::SUBJECT: a rig's id = its subject, `character.<id>[.<persona>]` or `item.<id>` */
export const SUBJECT = /^(?:character\.[a-z0-9][a-z0-9_-]{0,63}(?:\.[a-z0-9][a-z0-9_-]{0,63})?|item\.[A-Za-z0-9][A-Za-z0-9_-]{0,63})$/;

const ROOT = SCHEMA as unknown as SchemaRoot;
const DEF = { instruments: 'Instrument', presets: 'Preset', rigs: 'Rig' } as const;
/** stored next to the form, never served (the server strips it before deck-space's rules) */
const NOT_SERVED = ['accepted_warnings'];

/** scene.rs `split_ref`: `node.port`, a bare node = its `out` */
export function splitRef(ref: string): [string, string] {
	const at = ref.indexOf('.');
	return at < 0 ? [ref, 'out'] : [ref.slice(0, at), ref.slice(at + 1)];
}
/** instrument.rs `canonical`: `node` → `node.out` */
export const canonical = (ref: string) => splitRef(ref).join('.');

/** the serde form only (schema.json) */
export function formProblems(kind: RigKind, data: unknown): RigProblem[] {
	const served = data && typeof data === 'object' && !Array.isArray(data) ? Object.fromEntries(Object.entries(data).filter(([k]) => !NOT_SERVED.includes(k))) : data;
	return checkSchema(ROOT, { $ref: `#/$defs/${DEF[kind]}` }, served);
}

const nodeIds = (instrument: Data | null): Set<string> => new Set(Array.isArray(instrument?.node) ? instrument.node.map((n: Data) => n?.id).filter((id: unknown) => typeof id === 'string') : []);
const isObject = (v: unknown): v is Data => !!v && typeof v === 'object' && !Array.isArray(v);

export function instrumentProblems(data: Data, id: string): RigProblem[] {
	const who = `instrument ${id}`;
	const p: RigProblem[] = formProblems('instruments', data).map((x) => ({ path: x.path, message: `${who}: ${x.message}` }));
	if (!INSTRUMENT_ID.test(id)) p.push({ path: ['id'], message: `${who}: id must be lower-case letters, digits, _ or -` });
	if (data.id !== id) p.push({ path: ['id'], message: `${who}: id must be "${id}"` });
	if (p.length) return p;
	const seen = new Set<string>();
	data.node.forEach((n: Data, i: number) => {
		if (seen.has(n.id)) p.push({ path: ['node', i, 'id'], message: `${who}: node '${n.id}' twice` });
		seen.add(n.id);
	});
	const known = (ref: string) => seen.has(splitRef(ref)[0]);
	if (!known(data.out)) p.push({ path: ['out'], message: `${who}: out '${data.out}' names no node` });
	(data.link ?? []).forEach((l: Data, i: number) => {
		for (const end of ['from', 'to'] as const) if (!known(l[end])) p.push({ path: ['link', i, end], message: `${who}: link ${end} '${l[end]}' names no node` });
	});
	(data.expose ?? []).forEach((e: Data, i: number) => {
		if (!known(e.path)) p.push({ path: ['expose', i, 'path'], message: `${who}: no node '${splitRef(e.path)[0]}' (${e.path})` });
		if (Array.isArray(e.range) && e.range[0] > e.range[1]) p.push({ path: ['expose', i, 'range'], message: `${who}: ${e.path} range must be [min, max]` });
	});
	return p;
}

export function presetProblems(data: Data, id: string, lib: RigLibrary): RigProblem[] {
	const who = `preset ${id}`;
	const p: RigProblem[] = formProblems('presets', data).map((x) => ({ path: x.path, message: `${who}: ${x.message}` }));
	if (!PRESET_ID.test(id)) p.push({ path: ['id'], message: `${who}: id is not a valid preset id` });
	if (data.id !== id) p.push({ path: ['id'], message: `${who}: id must be "${id}"` });
	if (p.length) return p;
	const instrument = lib.instrument(data.instrument);
	if (!instrument) return [{ path: ['instrument'], message: `${who}: unknown instrument "${data.instrument}"` }];
	// preset.rs `lineage`: an unknown parent, a loop, a parent of another instrument
	const chain = [id];
	let child: Data = data;
	while (typeof child.parent === 'string') {
		const parent: string = child.parent;
		if (chain.includes(parent)) {
			p.push({ path: ['parent'], message: `${who}: its lineage loops at "${parent}"` });
			break;
		}
		const next = parent === id ? data : lib.preset(parent);
		if (!next) {
			p.push({ path: ['parent'], message: `${who}: unknown preset "${parent}" (the parent of '${chain[chain.length - 1]}')` });
			break;
		}
		if (next.instrument !== child.instrument) {
			p.push({ path: ['parent'], message: `${who}: '${chain[chain.length - 1]}' (${child.instrument}) derives from '${parent}' of another instrument (${next.instrument})` });
			break;
		}
		chain.push(parent);
		child = next;
	}
	const nodes = nodeIds(instrument);
	const node = (path: SchemaPath, ref: string, what: string) => {
		if (!nodes.has(splitRef(ref)[0])) p.push({ path, message: `${who}: ${what} ${ref} – instrument ${data.instrument} has no node '${splitRef(ref)[0]}'` });
	};
	for (const key of Object.keys(data.values ?? {})) node(['values', key], key, 'value');
	(data.unlink ?? []).forEach((u: string, i: number) => node(['unlink', i], u, 'unlink'));
	(data.link ?? []).forEach((l: Data, i: number) => (node(['link', i, 'from'], l.from, 'link from'), node(['link', i, 'to'], l.to, 'link to')));
	(data.mapping ?? []).forEach((m: Data, i: number) => node(['mapping', i, 'target'], m.target, 'mapping target'));
	return p;
}

/** the knobs a rig's sticks may name: every exposed parameter of an unmuted track, `<track>_<node>.<port>` (bundle.rs) */
export function rigKnobs(data: Data, lib: RigLibrary): string[] {
	const out: string[] = [];
	for (const t of Array.isArray(data.track) ? data.track : []) {
		if (!isObject(t) || t.mute === true || typeof t.id !== 'string') continue;
		const instrument = typeof t.instrument === 'string' ? lib.instrument(t.instrument) : null;
		for (const e of Array.isArray(instrument?.expose) ? instrument.expose : []) if (typeof e?.path === 'string') out.push(`${t.id}_${canonical(e.path)}`);
	}
	return out;
}

export function rigProblems(data: Data, id: string, lib: RigLibrary): RigProblem[] {
	const who = `rig ${id}`;
	const p: RigProblem[] = formProblems('rigs', data).map((x) => ({ path: x.path, message: `${who}: ${x.message}` }));
	if (!SUBJECT.test(id)) p.push({ path: ['subject'], message: `${who}: the id is a subject – character.<id>[.<persona>] or item.<id>` });
	if (data.subject !== id) p.push({ path: ['subject'], message: `${who}: subject must be "${id}"` });
	if (p.length) return p;
	// mod.rs `Rig::check`
	const tracks = new Map<string, Data>();
	const fits = (path: SchemaPath, at: string, track: Data, preset: string) => {
		const pr = lib.preset(preset);
		if (!pr) p.push({ path, message: `${at}: unknown preset "${preset}"` });
		else if (pr.instrument !== track.instrument) p.push({ path, message: `${at}: track '${track.id}' plays '${track.instrument}', preset '${preset}' is for '${pr.instrument}'` });
	};
	data.track.forEach((t: Data, i: number) => {
		if (tracks.has(t.id)) p.push({ path: ['track', i, 'id'], message: `${who}: track '${t.id}' twice` });
		if (/[:.]/.test(t.id)) p.push({ path: ['track', i, 'id'], message: `${who}: a track id has no ':' or '.' (${t.id})` });
		tracks.set(t.id, t);
		if (!lib.instrument(t.instrument)) p.push({ path: ['track', i, 'instrument'], message: `${who}: unknown instrument "${t.instrument}"` });
		if (typeof t.preset === 'string') fits(['track', i, 'preset'], who, t, t.preset);
	});
	for (const field of ['personas', 'moods'] as const) {
		const label = field === 'personas' ? 'persona' : 'mood';
		for (const [name, v] of Object.entries((data[field] ?? {}) as Record<string, Data>)) {
			const at = `${who}: ${label} ${name}`;
			for (const [track, preset] of Object.entries((v.presets ?? {}) as Record<string, string>)) {
				const t = tracks.get(track);
				if (!t) p.push({ path: [field, name, 'presets', track], message: `${at}: no track '${track}'` });
				else fits([field, name, 'presets', track], at, t, preset);
			}
			for (const path of Object.keys(v.values ?? {})) {
				const colon = path.indexOf(':');
				const t = colon < 0 ? undefined : tracks.get(path.slice(0, colon));
				if (colon < 0) p.push({ path: [field, name, 'values', path], message: `${at}: '${path}' is not <track>:<node>.<port>` });
				else if (!t) p.push({ path: [field, name, 'values', path], message: `${at}: no track '${path.slice(0, colon)}'` });
				else {
					const instrument = lib.instrument(t.instrument);
					const node = splitRef(path.slice(colon + 1))[0];
					if (instrument && !nodeIds(instrument).has(node)) p.push({ path: [field, name, 'values', path], message: `${at}: instrument ${t.instrument}: no node '${node}' (${path.slice(colon + 1)})` });
				}
			}
		}
	}
	(data.route ?? []).forEach((r: Data, i: number) => {
		for (const end of ['from', 'to'] as const) {
			const ref: string = r[end];
			const colon = ref.indexOf(':');
			if (colon < 0) p.push({ path: ['route', i, end], message: `${who}: route end '${ref}' has no scope` });
			else if (!(ref.slice(0, colon) === 'being' && end === 'to') && !tracks.has(ref.slice(0, colon))) p.push({ path: ['route', i, end], message: `${who}: no track '${ref.slice(0, colon)}' (route ${ref})` });
		}
	});
	if (data.sticks) {
		const knobs = rigKnobs(data, lib);
		(data.sticks as string[][]).forEach((pair, s) =>
			pair.forEach((path, axis) => {
				if (path !== '' && !knobs.includes(path)) p.push({ path: ['sticks', s, axis], message: `${who}: stick '${path}' is not one of its knobs` });
			})
		);
	}
	return p;
}

/** one entry of `kind` against the library */
export function rigEntryProblems(kind: RigKind, id: string, data: Data, lib: RigLibrary): RigProblem[] {
	// a tombstone (`removed: true`, D5) is not played – it leaves the catalog on publish
	if (data.removed === true) return [];
	if (kind === 'instruments') return instrumentProblems(data, id);
	if (kind === 'presets') return presetProblems(data, id, lib);
	return rigProblems(data, id, lib);
}

/** a library over lists of `{ id, data }` (the catalog documents as they stand; removed entries are gone) */
export function rigLibrary(instruments: { id: string; data: unknown }[], presets: { id: string; data: unknown }[]): RigLibrary {
	const map = (list: { id: string; data: unknown }[]) => new Map(list.filter((e) => isObject(e.data) && e.data.removed !== true).map((e) => [e.id, e.data as Data]));
	const i = map(instruments);
	const p = map(presets);
	return { instrument: (id) => i.get(id) ?? null, preset: (id) => p.get(id) ?? null };
}
