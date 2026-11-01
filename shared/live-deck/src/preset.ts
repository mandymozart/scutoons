// preset.ts (#270, research/INSTRUMENTS.md §2, §6): "save as preset" from the deck's knobs. A performer turns the knobs
// of a played rig (rig.ts) and keeps the result as their own derivative of the preset the track plays: this module
// works out which track a knob belongs to, what the performer changed against the played preset, and the body of
// `POST /api/rigs/presets` – the parent and only the changed values (the server drops what equals the parent's
// folded lineage besides, the same rule as deck-space's `PresetBook::derive`). Pure, no DOM, no fetch: the page sends it.
import type { RigKnob, RigPlayed } from './space.ts';

/** a played knob in its track: `<track>_<node>.<port>` → the track and the preset's path `node.port` */
export function knobTrack(p: RigPlayed, path: string): { track: string; port: string } | null {
	// the longest track id the path starts with (`air_hiss.gain` of the track `air`; ids may hold `_` themselves)
	const track = p.tracks
		.map((t) => t.track)
		.filter((t) => path.startsWith(`${t}_`))
		.sort((a, b) => b.length - a.length)[0];
	return track ? { track, port: path.slice(track.length + 1) } : null;
}

/** a track a preset can be saved for: it plays a preset (the parent) and shows knobs */
export interface SaveTrack {
	track: string;
	/** the preset the track plays – the derivative's parent */
	parent: string;
	knobs: RigKnob[];
}

/** the tracks of the played rig that can be saved from, in the rig's order (a track without a preset has no parent) */
export function saveTracks(p: RigPlayed): SaveTrack[] {
	const out: SaveTrack[] = [];
	for (const t of p.tracks) {
		if (!t.preset) continue;
		const knobs = p.knobs.filter((k) => knobTrack(p, k.path)?.track === t.track);
		if (knobs.length) out.push({ track: t.track, parent: t.preset, knobs });
	}
	return out;
}

/** one knob the performer moved: the preset's path, the played value and the knob's value now */
export interface PresetChange {
	/** `node.port` as the preset stores it */
	path: string;
	knob: RigKnob;
	from: number;
	to: number;
}

/** what changed on `track`: the knobs whose value now differs from the one the played preset gave them */
export function presetChanges(p: RigPlayed, track: string, values: Record<string, number>): PresetChange[] {
	const out: PresetChange[] = [];
	for (const k of p.knobs) {
		const at = knobTrack(p, k.path);
		if (at?.track !== track) continue;
		const now = values[k.path];
		if (typeof now !== 'number' || !Number.isFinite(now) || now === k.value) continue;
		out.push({ path: at.port, knob: k, from: k.value, to: now });
	}
	return out;
}

/** the body of `POST /api/rigs/presets` (server docs/API.md "Rigs") */
export interface DeriveBody {
	parent: string;
	name?: Record<string, string>;
	/** `node.port` → the value, only what the performer changed */
	values: Record<string, number>;
}

/**
 * The request that saves `track`'s knobs as the performer's derivative: the played preset as the parent, the changed
 * values, a name in `locale` when given. Null when the track plays no preset (nothing to derive from).
 */
export function deriveBody(p: RigPlayed, track: string, values: Record<string, number>, name = '', locale = 'en'): DeriveBody | null {
	const parent = p.tracks.find((t) => t.track === track)?.preset;
	if (!parent) return null;
	const body: DeriveBody = { parent, values: Object.fromEntries(presetChanges(p, track, values).map((c) => [c.path, c.to])) };
	const trimmed = name.trim().replace(/\s+/g, ' ');
	if (trimmed) body.name = { [locale]: trimmed.slice(0, 100) };
	return body;
}

/** a saved preset as the API answers it (deck-space's `Preset`, the fields the deck shows) */
export interface SavedPreset {
	id: string;
	instrument: string;
	parent?: string;
	name?: Record<string, string>;
	tags?: string[];
	values?: Record<string, number>;
}

/** the status line after a save: the name (or the id's tail) and how many values it keeps */
export function savedLine(p: SavedPreset, locale = 'en'): string {
	const name = p.name?.[locale] ?? p.name?.en ?? Object.values(p.name ?? {})[0] ?? p.id.slice(-6);
	const n = Object.keys(p.values ?? {}).length;
	return `SAVED ${name} · ${n} ${n === 1 ? 'VALUE' : 'VALUES'}`.toUpperCase();
}
