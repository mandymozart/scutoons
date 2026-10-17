// Copy / cut / paste in the creator (BACKLOG 10.39, agents/UI-CONCEPTS.md E.7): what goes on the clipboard and how a
// paste fits it into the open document. Pure functions (no Svelte, no DOM) – the runtime (the live region's handler,
// the DOM copy / cut / paste events, the menu) is clipboard.svelte.ts.
//
// The clipboard holds JSON with a marker (`CLIP_MARKER`): the copied entries (deep copies of the stored form – a text
// box, a placed item, a sample area, a slice event, VN events, slices, groups), the toon they were copied in and the
// sound library entries they use. One system only: no media travels – a paste into another toon REFERENCES the source
// toon's files (`../<toon>/…`, @scutoons/content `rebaseToonAsset`). Ids are fresh on paste: events lose theirs (the
// document's edit assigns new ones, event-ids.ts), text boxes get the target's next ids (a showTextBox event follows its
// box), placements a random one (`newPlacementId`).
import { rebaseToonAsset } from '@scutoons/content';

export const CLIP_MARKER = 'scutoons/creator-clip';

/** what a clip holds: one slice entry (canvas region), VN events, slices or groups */
export type ClipKind = 'dialog' | 'item' | 'sample' | 'event' | 'events' | 'slices' | 'groups';

export interface ClipSounds {
	/** chapter.audio.sampleLibrary entries the entries use (`{ id, path, variants? }`) */
	samples?: any[];
	/** chapter.audio.music.tracks entries the entries use */
	tracks?: any[];
}

export interface Clip {
	marker: typeof CLIP_MARKER;
	v: 1;
	kind: ClipKind;
	/** the toon it was copied in: the entries' media paths are relative to its folder */
	toon: string;
	/** the copied entries (stored form, deep copies) */
	entries: any[];
	/** the sound library entries the entries use – a paste adds the missing ones to the target chapter */
	sounds?: ClipSounds;
	/** where it came from (the toast: "from catpope ch. 3") */
	from?: { chapter?: number };
}

const copy = <T>(value: T): T => (value === undefined ? value : JSON.parse(JSON.stringify(value)));

export function makeClip(kind: ClipKind, toon: string, entries: readonly unknown[], extra: { sounds?: ClipSounds; chapter?: number } = {}): Clip {
	const sounds = extra.sounds && (extra.sounds.samples?.length || extra.sounds.tracks?.length) ? copy(extra.sounds) : undefined;
	return { marker: CLIP_MARKER, v: 1, kind, toon, entries: copy([...entries]), ...(sounds ? { sounds } : {}), ...(extra.chapter !== undefined ? { from: { chapter: extra.chapter } } : {}) };
}

export const clipText = (clip: Clip) => JSON.stringify(clip);

const KINDS: readonly ClipKind[] = ['dialog', 'item', 'sample', 'event', 'events', 'slices', 'groups'];

/** the clip in a clipboard text; null for anything else (plain text, another app's JSON) */
export function readClip(text: string | null | undefined): Clip | null {
	if (!text || !text.includes(CLIP_MARKER)) return null;
	try {
		const value = JSON.parse(text);
		if (value?.marker !== CLIP_MARKER || !KINDS.includes(value.kind) || !Array.isArray(value.entries) || typeof value.toon !== 'string') return null;
		return value as Clip;
	} catch {
		return null;
	}
}

// ── ids ────────────────────────────────────────────────────────────────────────────────────────────────────────

const ALPHABET = 'abcdefghijklmnopqrstuvwxyz0123456789';

/**
 * A placement id (BACKLOG 10.39 step 6): `<item>-<7 random>` – random like the event ids, so the same slice image
 * reused elsewhere in the toon never shares a pickup memory (`picked:<slice image>#<id>`). Existing ids stay.
 */
export function newPlacementId(item: string, taken: ReadonlySet<string> = new Set()): string {
	for (;;) {
		const bytes = new Uint8Array(7);
		crypto.getRandomValues(bytes);
		const id = `${item}-${[...bytes].map((b) => ALPHABET[b % ALPHABET.length]).join('')}`;
		if (!taken.has(id)) return id;
	}
}

type Obj = Record<string, any>;
const isObj = (v: unknown): v is Obj => !!v && typeof v === 'object' && !Array.isArray(v);
const list = (v: unknown): any[] => (Array.isArray(v) ? v : []);

/**
 * The list at `path` under `parent` (a slice: `['dialog']`, `['audio', 'audio', 'samples']` …), created with its
 * missing parents – each read back THROUGH `parent`: on the creator's $state document `slice.items ??= []` evaluates to
 * the raw array, and pushing into that changed the stored data without telling the canvas (owner 2026-10-17: a box /
 * item pasted into a slice without boxes / items showed in the tree, not on the slice).
 */
export function listAt(parent: Obj, path: readonly string[]): any[] {
	let at: any = parent;
	path.forEach((key, i) => {
		if (at[key] === undefined || at[key] === null) at[key] = i === path.length - 1 ? [] : {};
		at = at[key];
	});
	return at;
}

/** pasted events lose their id: the document's edit gives them fresh ones (assignEventIds) */
export function dropEventIds(events: unknown): void {
	for (const e of list(events)) if (isObj(e)) delete e.id;
}

/** a slice's events, its placements' events: ids dropped; placements get random ids */
function freshSlice(slice: Obj, placementIds: Set<string>): void {
	dropEventIds(slice.events);
	for (const item of list(slice.items)) {
		if (!isObj(item)) continue;
		dropEventIds(item.events);
		item.id = newPlacementId(String(item.item ?? 'item'), placementIds);
		placementIds.add(item.id);
	}
}

// ── media across toons ─────────────────────────────────────────────────────────────────────────────────────────

/** a slice's media paths (image, its levels, the video + its levels / sources) as toon `to` stores them */
export function rebaseSliceMedia(slice: Obj, from: string, to: string): void {
	if (from === to || !isObj(slice)) return;
	const re = (p: unknown) => (typeof p === 'string' && p ? rebaseToonAsset(p, from, to) : p);
	if (slice.image) slice.image = re(slice.image);
	if (isObj(slice.imageVariants)) for (const k of Object.keys(slice.imageVariants)) slice.imageVariants[k] = re(slice.imageVariants[k]);
	const video = slice.video;
	if (isObj(video)) {
		if (video.src) video.src = re(video.src);
		if (video.poster) video.poster = re(video.poster);
		if (isObj(video.variants)) for (const k of Object.keys(video.variants)) video.variants[k] = re(video.variants[k]);
		for (const s of list(video.sources)) if (isObj(s) && s.src) s.src = re(s.src);
	}
}

/** how many files of the clip's toon a paste into `to` references (the toast: "uses 4 files from catpope") */
export function foreignFiles(entries: readonly any[], sounds: ClipSounds | undefined, from: string, to: string): number {
	if (from === to) return 0;
	const files = new Set<string>();
	const add = (p: unknown) => typeof p === 'string' && p && files.add(p);
	const slices = (s: any) => {
		if (!isObj(s)) return;
		add(s.image);
		Object.values(isObj(s.imageVariants) ? s.imageVariants : {}).forEach(add);
		if (isObj(s.video)) {
			add(s.video.src);
			Object.values(isObj(s.video.variants) ? s.video.variants : {}).forEach(add);
		}
	};
	for (const e of entries) {
		if (isObj(e) && Array.isArray(e.slices)) e.slices.forEach(slices);
		else slices(e);
	}
	for (const s of [...list(sounds?.samples), ...list(sounds?.tracks)]) {
		if (!isObj(s)) continue;
		add(s.path);
		Object.values(isObj(s.variants) ? s.variants : {}).forEach(add);
	}
	return files.size;
}

// ── sounds ─────────────────────────────────────────────────────────────────────────────────────────────────────

/** the sample / music ids slices use (sample areas, music levels) */
function soundIds(slices: readonly any[]): { samples: Set<string>; tracks: Set<string> } {
	const samples = new Set<string>();
	const tracks = new Set<string>();
	for (const s of slices) {
		for (const x of list(s?.audio?.audio?.samples)) if (typeof x?.id === 'string') samples.add(x.id);
		for (const t of list(s?.audio?.music?.tracks)) if (typeof t?.id === 'string') tracks.add(t.id);
	}
	return { samples, tracks };
}

/** the library entries `slices` use, from the chapter's audio (`chapter.audio`) */
export function soundsOf(slices: readonly any[], audio: any): ClipSounds {
	const ids = soundIds(slices);
	return {
		samples: list(audio?.sampleLibrary).filter((s) => ids.samples.has(s?.id)),
		tracks: list(audio?.music?.tracks).filter((t) => ids.tracks.has(t?.id))
	};
}

/**
 * Adds the clip's sound entries the target chapter lacks (`file` = the chapter document, mutated) – an entry with the
 * same id and file is the same sound; the same id with another file gets a new id. Their paths are rebased when the
 * toon differs. Returns the id changes (old → new) for `remapSounds`.
 */
export function mergeSounds(file: Obj, sounds: ClipSounds | undefined, from: string, to: string): { samples: Map<string, string>; tracks: Map<string, string> } {
	const out = { samples: new Map<string, string>(), tracks: new Map<string, string>() };
	if (!sounds) return out;
	const rebase = (entry: Obj) => {
		const e = copy(entry);
		if (typeof e.path === 'string') e.path = rebaseToonAsset(e.path, from, to);
		if (typeof e.src === 'string') e.src = rebaseToonAsset(e.src, from, to);
		if (isObj(e.variants)) for (const k of Object.keys(e.variants)) if (typeof e.variants[k] === 'string') e.variants[k] = rebaseToonAsset(e.variants[k], from, to);
		return e;
	};
	const merge = (target: any[], entries: any[], map: Map<string, string>) => {
		for (const raw of entries) {
			if (!isObj(raw) || typeof raw.id !== 'string') continue;
			const entry = rebase(raw);
			const same = target.find((t) => t?.id === entry.id);
			if (same && (same.path ?? same.src) === (entry.path ?? entry.src)) continue;
			const sameFile = target.find((t) => (t?.path ?? t?.src) && (t.path ?? t.src) === (entry.path ?? entry.src));
			if (sameFile) {
				map.set(entry.id, sameFile.id);
				continue;
			}
			let id = entry.id;
			for (let n = 2; target.some((t) => t?.id === id); n++) id = `${entry.id}_${n}`;
			if (id !== entry.id) map.set(entry.id, id);
			target.push({ ...entry, id });
		}
	};
	if (list(sounds.samples).length) {
		file.audio ??= {};
		file.audio.sampleLibrary ??= [];
		merge(file.audio.sampleLibrary, list(sounds.samples), out.samples);
	}
	if (list(sounds.tracks).length) {
		file.audio ??= {};
		file.audio.music ??= {};
		file.audio.music.tracks ??= [];
		merge(file.audio.music.tracks, list(sounds.tracks), out.tracks);
	}
	return out;
}

/** a pasted slice's sample / music ids follow `mergeSounds`' changes */
export function remapSounds(slice: Obj, map: { samples: Map<string, string>; tracks: Map<string, string> }): void {
	for (const x of list(slice?.audio?.audio?.samples)) if (isObj(x) && map.samples.has(x.id)) x.id = map.samples.get(x.id);
	for (const t of list(slice?.audio?.music?.tracks)) if (isObj(t) && map.tracks.has(t.id)) t.id = map.tracks.get(t.id);
}

// ── text boxes ─────────────────────────────────────────────────────────────────────────────────────────────────

/** every showTextBox event of a group (its script, its slices' events, their placements' events) */
function showTextBoxEvents(group: Obj): Obj[] {
	const out: Obj[] = [];
	const scan = (events: unknown) => list(events).forEach((e) => isObj(e) && e.type === 'showTextBox' && isObj(e.params) && out.push(e));
	scan(group.events);
	for (const s of list(group.slices)) {
		if (!isObj(s)) continue;
		scan(s.events);
		for (const it of list(s.items)) if (isObj(it)) scan(it.events);
	}
	return out;
}

/**
 * Pasted slices (of one group, or a group): every text box gets a fresh id (`nextId(slice, its index in the list)`,
 * the target's next free id), and every showTextBox event among them follows its box.
 */
export function renameBoxes(group: Obj, nextId: (slice: Obj, index: number) => string): Map<string, string> {
	const map = new Map<string, string>();
	list(group.slices).forEach((s, i) => {
		for (const d of list(s?.dialog)) {
			if (!isObj(d)) continue;
			const id = nextId(s, i);
			if (typeof d.id === 'string' && d.id) map.set(d.id, id);
			d.id = id;
		}
	});
	for (const e of showTextBoxEvents(group)) if (map.has(e.params.dialog)) e.params.dialog = map.get(e.params.dialog);
	return map;
}

// ── pasting slices / groups ────────────────────────────────────────────────────────────────────────────────────

export interface PasteContext {
	/** the toon pasted into */
	toon: string;
	/** placement ids already in the slices pasted into (new random ones avoid them) */
	placementIds?: Set<string>;
	/** the next free text box id for a pasted slice (its index among the pasted ones) */
	nextBoxId: (slice: Obj, index: number) => string;
}

/** clip slices made ready for `ctx.toon`: media rebased, fresh ids (boxes, events, placements), showTextBox kept in step */
export function prepareSlices(clip: Clip, slices: readonly any[], ctx: PasteContext): Obj[] {
	const out = copy(slices.filter(isObj)) as Obj[];
	const placements = ctx.placementIds ?? new Set<string>();
	for (const s of out) {
		rebaseSliceMedia(s, clip.toon, ctx.toon);
		freshSlice(s, placements);
	}
	renameBoxes({ slices: out }, ctx.nextBoxId);
	return out;
}

/**
 * A clip group made ready for `ctx.toon`: as `prepareSlices` for its slices, its script's ids dropped (a showTextBox in
 * the script follows its box too), the label made unique among `labels` (the chapter's group labels).
 */
export function prepareGroup(clip: Clip, group: any, ctx: PasteContext, labels: Set<string>): Obj {
	const g = copy(group) as Obj;
	const placements = ctx.placementIds ?? new Set<string>();
	for (const s of list(g.slices)) {
		if (!isObj(s)) continue;
		rebaseSliceMedia(s, clip.toon, ctx.toon);
		freshSlice(s, placements);
	}
	dropEventIds(g.events);
	renameBoxes(g, ctx.nextBoxId);
	if (typeof g.label === 'string' && g.label) {
		let label = g.label;
		for (let n = 2; labels.has(label); n++) label = `${g.label}-${n}`;
		g.label = label;
		labels.add(label);
	}
	return g;
}

// ── settings only (Shift+V) ────────────────────────────────────────────────────────────────────────────────────

/** a text box's own keys – everything else is its look (variant, frame, shape, tail, font, stroke, rotation, scale …) */
const BOX_OWN = new Set(['id', 'rect', 'anchor', 'text', 'layer']);

/** Shift+V on a text box: the copied box's look, its words, id, place (rect + anchor) and layer stay */
export function applyBoxLook(target: Obj, source: Obj): void {
	for (const key of Object.keys(target)) if (!BOX_OWN.has(key)) delete target[key];
	for (const [key, value] of Object.entries(source)) if (!BOX_OWN.has(key)) target[key] = copy(value);
}

/** what a placement does (Shift+V on a placed item): action, qty, repeat, respawn, its events */
const ITEM_SETTINGS = ['action', 'qty', 'repeat', 'respawn', 'events'] as const;

export function applyItemSettings(target: Obj, source: Obj): void {
	for (const key of ITEM_SETTINGS) {
		if (source[key] === undefined) delete target[key];
		else target[key] = copy(source[key]);
	}
	dropEventIds(target.events);
}

/** Shift+V on a sample area: the sound, its trigger + params; the area (rect) stays */
export function applySampleSettings(target: Obj, source: Obj): void {
	const rect = target.trigger?.rect;
	for (const key of Object.keys(target)) delete target[key];
	Object.assign(target, copy(source));
	if (rect) {
		target.trigger ??= {};
		target.trigger.rect = rect;
	}
	else if (isObj(target.trigger)) delete target.trigger.rect;
}

/** Shift+V on a slice event: the copied event's type, params, `if` and trigger; its own id and area stay */
export function applySliceEventSettings(target: Obj, source: Obj): void {
	const id = target.id;
	const rect = target.trigger?.rect;
	for (const key of Object.keys(target)) delete target[key];
	Object.assign(target, copy(source));
	if (id) target.id = id;
	else delete target.id;
	if (isObj(target.trigger)) {
		if (rect) target.trigger.rect = rect;
		else delete target.trigger.rect;
	}
}

/** Shift+V on VN events: params + `if` of the first copied event of the same type; false = no such type in the clip */
export function applyVnSettings(target: Obj, sources: readonly any[]): boolean {
	const source = sources.find((s) => isObj(s) && s.type === target.type);
	if (!source) return false;
	if (source.params === undefined) delete target.params;
	else target.params = copy(source.params);
	if (source.if === undefined) delete target.if;
	else target.if = copy(source.if);
	return true;
}

/** Shift+V on slices: the copied slice's audio + events (fresh ids); image, boxes and items stay */
export function applySliceSettings(target: Obj, source: Obj): void {
	for (const key of ['audio', 'events'] as const) {
		if (source[key] === undefined) delete target[key];
		else target[key] = copy(source[key]);
	}
	dropEventIds(target.events);
}

/** Shift+V on groups: the copied group's renderer, end and `continues` */
export function applyGroupSettings(target: Obj, source: Obj): void {
	for (const key of ['renderer', 'end', 'continues'] as const) {
		if (source[key] === undefined) delete target[key];
		else target[key] = copy(source[key]);
	}
}

// ── item limits ────────────────────────────────────────────────────────────────────────────────────────────────

/** placements per catalog item in a chapter / scene document (or anything with groups / slices) */
export function countPlacements(doc: unknown): Map<string, number> {
	const out = new Map<string, number>();
	const slices = (g: any) => list(g?.slices);
	const groups = isObj(doc) && Array.isArray(doc.groups) ? doc.groups : [doc];
	for (const g of groups) for (const s of slices(g)) for (const it of list(s?.items)) if (typeof it?.item === 'string') out.set(it.item, (out.get(it.item) ?? 0) + 1);
	return out;
}

/** the catalog items a clip would place (+ how many of each) */
export function clipPlacements(clip: Clip): Map<string, number> {
	if (clip.kind === 'item') {
		const out = new Map<string, number>();
		for (const it of clip.entries) if (typeof it?.item === 'string') out.set(it.item, (out.get(it.item) ?? 0) + 1);
		return out;
	}
	if (clip.kind === 'slices') return countPlacements({ slices: clip.entries });
	if (clip.kind === 'groups') return countPlacements({ groups: clip.entries });
	return new Map();
}
