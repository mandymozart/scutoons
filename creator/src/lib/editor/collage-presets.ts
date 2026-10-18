// Custom collage presets in the piece inspector (agents/research/COLLAGE.md §12.4) – the pure half: what the look grid
// offers, whether a piece draws differently from what it names, a new preset's values, name and id, and the edits of
// the presets document `colorations` (presets-doc.ts). preset-actions.ts runs them behind its dialogs and toasts.
// The document is a $state proxy of another page's document: every value taken from it or put into it is a JSON copy.
import {
	COLLAGE_LOOKS,
	PIECE_STYLE_KEYS,
	PRESET_ID,
	pieceStyle,
	slugId,
	type CollagePiece,
	type CollagePreset,
	type Colorations,
	type PieceStyle,
	type PresetList
} from '@scutoons/content';

/**
 * How many live custom presets the grid holds; one more and all of them sit in the Custom fold under it. Measured
 * 2026-10-18 with the preview tiles (look-grid.svelte, ≥ 4.5rem): four a row in the 22rem panel, five on a 375px
 * phone – As drawn + the ten + 5 take four rows on both (research/COLLAGE.md §12.4 guessed 6)
 */
export const GRID_PRESETS = 5;
/** a preset name's and a preset id's longest form */
export const NAME_MAX = 40;
const ID_MAX = 40;
/** ids a custom preset never takes: the code looks' and "As drawn"'s testid ('none') – PRESET_ID refuses them too */
const RESERVED = [...COLLAGE_LOOKS.map((look) => look.id), 'none'];

const copy = <T>(value: T): T => JSON.parse(JSON.stringify(value)) as T;
const isObj = (v: unknown): v is Record<string, unknown> => !!v && typeof v === 'object' && !Array.isArray(v);

/** a grid cell of a custom preset */
export interface PresetChoice {
	id: string;
	/** its name (a stored entry without one: its id) */
	name: string;
	/** deleted – offered only to the piece that names it ("Night (deleted)") */
	removed: boolean;
}

/** the stored presets a piece can name: an id per PRESET_ID, the first of a repeated id (= @scutoons/content customPreset) */
function entries(presets: PresetList): CollagePreset[] {
	if (!Array.isArray(presets)) return [];
	const seen = new Set<string>();
	return presets.filter((p: unknown): p is CollagePreset => {
		if (!isObj(p) || typeof p.id !== 'string' || !PRESET_ID.test(p.id) || seen.has(p.id)) return false;
		seen.add(p.id);
		return true;
	});
}

const nameOf = (p: CollagePreset) => (typeof p.name === 'string' && p.name.trim() ? p.name.trim() : p.id);

/**
 * The custom presets the look grid offers after the ten, in stored order: the live ones, and a deleted one only where
 * this piece names it. `inGrid` = they are cells of the grid; false from the GRID_PRESETS + 1-th live preset on – then
 * all of them sit in the Custom fold under it
 */
export function presetChoices(presets: PresetList, current: string | undefined): { choices: PresetChoice[]; inGrid: boolean } {
	const shown = entries(presets).filter((p) => p.removed !== true || p.id === current);
	const live = shown.filter((p) => p.removed !== true).length;
	return { choices: shown.map((p) => ({ id: p.id, name: nameOf(p), removed: p.removed === true })), inGrid: live <= GRID_PRESETS };
}

/** a value with its keys sorted (two styles compared regardless of their key order) */
function canonical(value: unknown): unknown {
	if (Array.isArray(value)) return value.map(canonical);
	if (!isObj(value)) return value;
	return Object.fromEntries(Object.keys(value).sort().map((k) => [k, canonical(value[k])]));
}
const same = (a: unknown, b: unknown) => JSON.stringify(canonical(a)) === JSON.stringify(canonical(b));

/** whether the piece draws differently from what it names (its own values change something; none named = as drawn) */
export function ownDiffers(piece: Pick<CollagePiece, 'preset' | 'style'>, presets: PresetList): boolean {
	return !same(pieceStyle(piece, presets), pieceStyle({ preset: piece.preset }, presets));
}

/** the piece as drawn, as a preset's values: a deep copy, nulls dropped (an effect switched off stays off – no key) */
export function presetValues(piece: Pick<CollagePiece, 'preset' | 'style'>, presets: PresetList): PieceStyle {
	return JSON.parse(JSON.stringify(pieceStyle(piece, presets), (_key, value) => (value === null ? undefined : value))) as PieceStyle;
}

/**
 * A preset's name: trimmed, at most NAME_MAX characters, `fallback` when empty ("Preset 3"); a name the grid already
 * shows – a live preset's (but `except`'s, a rename), or one of `also` (the code looks' names) – gets " 2", " 3" …
 * (case-insensitive)
 */
export function presetName(typed: string, fallback: string, presets: PresetList, options: { except?: string; also?: readonly string[] } = {}): string {
	const base = (typed.trim() || fallback.trim()).slice(0, NAME_MAX).trim();
	const shown = new Set([...entries(presets).filter((p) => p.removed !== true && p.id !== options.except).map(nameOf), ...(options.also ?? [])].map((n) => n.toLowerCase()));
	if (!shown.has(base.toLowerCase())) return base;
	for (let i = 2; ; i++) {
		const suffix = ` ${i}`;
		const name = `${base.slice(0, NAME_MAX - suffix.length).trimEnd()}${suffix}`;
		if (!shown.has(name.toLowerCase())) return name;
	}
}

/** the name a new preset gets when none is typed: "Preset n" – n after the live presets (`word` = "Preset {n}" localised) */
export const placeholderName = (presets: PresetList, word: (n: number) => string) => word(entries(presets).filter((p) => p.removed !== true).length + 1);

/**
 * A new preset's id from its name: the slug (made to start with a letter), at most ID_MAX characters, and never one a
 * stored entry has (deleted ones included – pieces may still name them) or a code look's / 'none' (`-2`, `-3` …)
 */
export function presetIdFor(name: string, presets: PresetList): string {
	let base = slugId(name);
	if (!/^[a-z]/.test(base)) base = `p-${base}`;
	base = base.slice(0, ID_MAX).replace(/-+$/, '');
	const stored = Array.isArray(presets) ? presets.flatMap((p: unknown) => (isObj(p) && typeof p.id === 'string' ? [p.id] : [])) : [];
	const taken = new Set([...RESERVED, ...stored]);
	for (let i = 1; ; i++) {
		const id = i === 1 ? base : `${base.slice(0, ID_MAX - String(i).length - 1).replace(/-+$/, '')}-${i}`;
		if (!taken.has(id) && PRESET_ID.test(id)) return id;
	}
}

// ── the presets document's edits (inside `presets.edit`) ───────────────────────────────────────────────────────────

function list(file: Colorations): CollagePreset[] {
	if (!Array.isArray(file.presets)) file.presets = [];
	return file.presets;
}
/** the entry a piece naming `id` draws (the first with that id) */
const entryOf = (file: Colorations, id: string): CollagePreset | undefined => entries(list(file)).find((p) => p.id === id);

/** only the six style keys of a value (anything else stays out) */
function styleKeys(values: PieceStyle): PieceStyle {
	const out: Record<string, unknown> = {};
	for (const key of PIECE_STYLE_KEYS) if (values[key] !== undefined && values[key] !== null) out[key] = copy(values[key]);
	return out as PieceStyle;
}

/** a preset's six style keys, a copy (what an Update's Undo puts back) */
export const styleOfPreset = (file: Colorations, id: string): PieceStyle | undefined => {
	const preset = entryOf(file, id);
	return preset && styleKeys(preset);
};

/** a new preset, newest last (the grid's order) */
export function addPreset(file: Colorations, preset: { id: string; name: string } & PieceStyle): void {
	list(file).push({ id: preset.id, name: preset.name, ...styleKeys(preset) });
}

/** a preset's values replaced by `values` (the six keys); its id, name and `removed` stay. false = no such preset */
export function updatePreset(file: Colorations, id: string, values: PieceStyle): boolean {
	const preset = entryOf(file, id);
	if (!preset) return false;
	for (const key of PIECE_STYLE_KEYS) delete preset[key];
	Object.assign(preset, styleKeys(values));
	return true;
}

/** a preset's name (its id never changes – pieces name it) */
export function renamePreset(file: Colorations, id: string, name: string): boolean {
	const preset = entryOf(file, id);
	if (!preset) return false;
	preset.name = name;
	return true;
}

/** deleted = a tombstone: it leaves the list, every piece that names it still draws it, its id stays taken (§12.2) */
export function removePreset(file: Colorations, id: string): boolean {
	const preset = entryOf(file, id);
	if (!preset) return false;
	preset.removed = true;
	return true;
}

/** a deleted preset back in the list (a Delete's Undo) */
export function restorePreset(file: Colorations, id: string): boolean {
	const preset = entryOf(file, id);
	if (!preset) return false;
	delete preset.removed;
	return true;
}

/** a preset taken out of the document – only a Save's Undo, right after it was added (no piece names it then) */
export function dropPreset(file: Colorations, id: string): boolean {
	const at = list(file).findIndex((p) => isObj(p) && p.id === id);
	if (at < 0) return false;
	list(file).splice(at, 1);
	return true;
}

// ── the piece's side ──────────────────────────────────────────────────────────────────────────────────────────────

/** what a piece names and its own values – a copy (what an Undo gives the piece back) */
export type PieceLook = Pick<CollagePiece, 'preset' | 'style'>;
export const pieceLook = (piece: PieceLook): PieceLook => copy({ preset: piece.preset, style: piece.style });

/** the piece names `look.preset` again, with its own values `look.style` (none = no key) */
export function setPieceLook(piece: CollagePiece, look: PieceLook): void {
	if (look.preset) piece.preset = look.preset;
	else delete piece.preset;
	if (look.style && Object.keys(look.style).length) piece.style = copy(look.style);
	else delete piece.style;
}
