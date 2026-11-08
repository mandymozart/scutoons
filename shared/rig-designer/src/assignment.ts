// The Assignment tab's document edits (#283, RIG-EDITING.md §2 tab 4): a character's rig (`rig:character.<id>`, deck-space
// `rig::Rig` in its JSON form) changed by the form – tracks (an instrument and its default preset, the mixer strip) and the
// presets per mood (the rig's mood variants, which the core replays on every mood, #368). Authoring state only: whether
// the rig plays, which characters lack one and the bundle the deck hears are the core's (space-web `rig_coverage`,
// `rig_catalog_bundle`, `rig_play`); the API checks the document on every write (#313). Every edit returns a new rig.

/** a rig as the catalog stores it (deck-space `rig::Rig`); fields the form does not edit travel along untouched */
export interface RigDoc {
	subject: string;
	track: RigTrack[];
	route?: { from: string; to: string; scale?: number; offset?: number }[];
	personas?: Record<string, RigVariant>;
	moods?: Record<string, RigVariant>;
	/** a code rig: the `module:<id>` document that drives the tracks (#412) */
	policy?: { module: string; seed?: number; values?: Record<string, number> };
	[other: string]: unknown;
}

/** a rig's face as the core describes it (deck-space `rig::face::RigFace`, space-web `rig_face`): the editor only shows it */
export interface RigFace {
	subject: string;
	tracks: FaceTrack[];
	policy?: { module: string };
	personas: string[];
	moods: string[];
}

export interface FaceTrack {
	id: string;
	instrument: string;
	preset?: string;
	mute?: boolean;
	presets?: string[];
	/** the policy output that chooses its preset (`<track>:@preset`) */
	preset_driven?: RigDriven;
	/** the policy output that writes its mixer strip (`<track>:@gain`) */
	gain_driven?: RigDriven;
	addressable: { path: string; input: string; min: number; max: number; log?: boolean; driven?: RigDriven }[];
}

/** a policy output on an input; `owned`: the core says the form does not write it */
export interface RigDriven {
	output: string;
	mode: string;
	owned: boolean;
}

export interface RigTrack {
	id: string;
	instrument: string;
	preset?: string;
	gain?: number;
	pan?: number;
	mute?: boolean;
}

/** a persona's or a mood's overrides: another preset per track, single values `<track>:<node>.<port>` */
export interface RigVariant {
	presets?: Record<string, string>;
	values?: Record<string, number>;
}

/** a catalog instrument / preset as the pickers list it (the documents' own fields) */
export interface CatalogInstrument {
	id: string;
	kind?: string;
	name?: Record<string, string>;
}
export interface CatalogPreset {
	id: string;
	instrument: string;
	name?: Record<string, string>;
}

/** how a character's default rig stands (deck-space `rig::assign::Coverage`) */
export type RigStatus = 'plays' | 'drafted' | 'missing';
export interface RigCoverage {
	character: string;
	subject: string;
	default: RigStatus;
	personas?: Record<string, RigStatus>;
	missing: boolean;
}

/** the module the rig's policy names (null: none, a declarative rig) */
export const policyOf = (rig: RigDoc | null): string | null => rig?.policy?.module ?? null;

/** the rig driven by `module:<module>` (what a policy's seed and values were stays with it) */
export const setPolicy = (rig: RigDoc, module: string): RigDoc => ({ ...rig, policy: { ...rig.policy, module } });

/** what the face says of a track's row */
export const faceTrack = (face: RigFace | null, id: string): FaceTrack | undefined => face?.tracks.find((t) => t.id === id);

/** the driven the form may not write, else null */
export const ownedBy = (driven?: RigDriven): RigDriven | null => (driven?.owned ? driven : null);

/** the inputs the policy owns (the rows the form has no field for), with who drives them */
export const ownedInputs = (face: RigFace | null): { path: string; by: RigDriven }[] =>
	(face?.tracks ?? []).flatMap((t) => t.addressable.flatMap((a) => (a.driven?.owned ? [{ path: a.path, by: a.driven }] : [])));

/** a new rig of `subject`: no tracks yet */
export const emptyRig = (subject: string): RigDoc => ({ subject, track: [] });

/** a track id from `base` (an instrument's id) the rig does not use yet – no `:` or `.` (the core's paths split on them) */
export function trackId(rig: RigDoc, base: string): string {
	const stem = base.replace(/[^a-z0-9_-]+/gi, '_') || 'track';
	const used = new Set(rig.track.map((t) => t.id));
	if (!used.has(stem)) return stem;
	let n = 2;
	while (used.has(`${stem}${n}`)) n++;
	return `${stem}${n}`;
}

/** a track playing `instrument` (its first preset as the default, if it has one) at the end */
export function addTrack(rig: RigDoc, instrument: string, presets: CatalogPreset[]): RigDoc {
	const preset = presetsOf(instrument, presets)[0]?.id;
	return { ...rig, track: [...rig.track, { id: trackId(rig, instrument), instrument, ...(preset ? { preset } : {}) }] };
}

/** the presets a track playing `instrument` can pick (the picker's list; the core refuses another instrument's) */
export const presetsOf = (instrument: string, presets: CatalogPreset[]) => presets.filter((p) => p.instrument === instrument);

/** a track's fields changed; another instrument drops the presets of the old one (the track's and the variants') */
export function setTrack(rig: RigDoc, id: string, change: Partial<Omit<RigTrack, 'id'>>): RigDoc {
	const old = rig.track.find((t) => t.id === id);
	if (!old) return rig;
	const next: RigTrack = { ...old, ...change };
	if (change.instrument !== undefined && change.instrument !== old.instrument && !('preset' in change)) delete next.preset;
	if (next.preset === undefined || next.preset === '') delete next.preset;
	const swapped = next.instrument !== old.instrument;
	const out: RigDoc = { ...rig, track: rig.track.map((t) => (t.id === id ? next : t)) };
	return swapped ? withVariants(out, (v) => dropTrack(v, id)) : out;
}

/** the track gone, and what names it: the variants' presets and values, the routes */
export function removeTrack(rig: RigDoc, id: string): RigDoc {
	const out: RigDoc = withVariants({ ...rig, track: rig.track.filter((t) => t.id !== id) }, (v) => dropTrack(v, id));
	if (rig.route) out.route = rig.route.filter((r) => !r.from.startsWith(`${id}:`) && !r.to.startsWith(`${id}:`));
	return out;
}

/** the preset `track` plays in `mood` (null: the track's own) */
export const moodPreset = (rig: RigDoc, mood: string, track: string): string | null => rig.moods?.[mood]?.presets?.[track] ?? null;

/** `track` plays `preset` in `mood`; null or '' = the track's own again (an empty mood is dropped) */
export function setMoodPreset(rig: RigDoc, mood: string, track: string, preset: string | null): RigDoc {
	const moods = { ...(rig.moods ?? {}) };
	const v: RigVariant = { ...(moods[mood] ?? {}), presets: { ...(moods[mood]?.presets ?? {}) } };
	if (preset) v.presets![track] = preset;
	else delete v.presets![track];
	moods[mood] = tidy(v);
	return withMoods(rig, moods);
}

/** the moods the form shows: the rig's own, then the ones offered (the character's) it has none for */
export function moodRows(rig: RigDoc, offered: string[]): string[] {
	const own = Object.keys(rig.moods ?? {});
	return [...own, ...offered.filter((m) => !own.includes(m))];
}

function dropTrack(v: RigVariant, id: string): RigVariant {
	const presets = { ...(v.presets ?? {}) };
	delete presets[id];
	const values = Object.fromEntries(Object.entries(v.values ?? {}).filter(([path]) => !path.startsWith(`${id}:`)));
	return tidy({ ...v, presets, values });
}

/** empty maps left out, as the core writes them */
function tidy(v: RigVariant): RigVariant {
	const out: RigVariant = { ...v };
	if (!out.presets || Object.keys(out.presets).length === 0) delete out.presets;
	if (!out.values || Object.keys(out.values).length === 0) delete out.values;
	return out;
}

function withVariants(rig: RigDoc, f: (v: RigVariant) => RigVariant): RigDoc {
	const map = (vs?: Record<string, RigVariant>) => (vs ? Object.fromEntries(Object.entries(vs).map(([k, v]) => [k, f(v)])) : undefined);
	const out: RigDoc = { ...rig };
	if (rig.personas) out.personas = dropEmpty(map(rig.personas)!);
	if (out.personas && Object.keys(out.personas).length === 0) delete out.personas;
	if (rig.moods) return withMoods(out, map(rig.moods)!);
	return out;
}

function withMoods(rig: RigDoc, moods: Record<string, RigVariant>): RigDoc {
	const out: RigDoc = { ...rig, moods: dropEmpty(moods) };
	if (Object.keys(out.moods!).length === 0) delete out.moods;
	return out;
}

const dropEmpty = (vs: Record<string, RigVariant>) => Object.fromEntries(Object.entries(vs).filter(([, v]) => v.presets || v.values));
