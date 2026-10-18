// The version note's suggestion (owner 2026-10-18: "for the notes in both, I'd like an automatic suggestion every time
// that fits what was last done, because typing something is really tedious", translated – DESIGN.md "Version notes").
// Pure and deterministic, no AI call: `chapterChanges` / `groupChanges` / `metaChanges` diff the stored document against
// the edited one into language-free records; `summarizeChanges` groups them by kind and writes the line in the UI
// language through `NoteWords` (version-note-words.ts gives the paraglide words, the tests their own). Texts of a
// language-neutral document are locale maps (agents/NEUTRAL.md): a change only in a translation is "texts (en)
// translated", not an edit of the box.
import { pieceName } from '@scutoons/content';

export type NoteKind = 'textbox' | 'item' | 'piece' | 'tail' | 'slice' | 'group' | 'event' | 'sound' | 'text' | 'settings' | 'entry' | 'created';
export type NoteAction = 'added' | 'removed' | 'moved' | 'recoloured' | 'set' | 'changed' | 'reordered' | 'translated';
/** a group as the note names it: a scene by its name, a chapter's group by its label, else its number */
export type GroupRef = { scene: string } | { label: string } | { number: number };
export interface NotePlace {
	group?: GroupRef;
	/** 1-based, within its group (the editor's "Slice n / m") */
	slice?: number;
}
export interface NoteChange {
	kind: NoteKind;
	action: NoteAction;
	place?: NotePlace;
	/** a single thing's name: a text box's first words, an item's / piece's / entry's name */
	name?: string;
	/** translated: the language; settings: the field */
	lang?: string;
	field?: string;
}
/** the text languages of a neutral document (its locale maps), the source first – null: a document without locale maps */
export type NoteLanguages = { source: string; languages: readonly string[] } | null;
export interface NoteContext {
	languages: NoteLanguages;
	/** an item's name (the catalog's); none = its id */
	itemName?: (id: string) => string;
	/** a collage piece's name; none = `pieceName` (its file / character) */
	pieceName?: (piece: any) => string;
}

/** the UI language's words; `summarizeChanges` only composes them */
export interface NoteWords {
	/** "Textbox", "3 Textboxen"; a slice: "Slice 4" (`name` = its number) / "3 Slices" */
	subject(kind: NoteKind, count: number): string;
	/** "verschoben"; an item added is "platziert" */
	verb(action: NoteAction, kind: NoteKind): string;
	/** "{subject}{place} {verb}" (de) / "{subject} {verb}{place}" (en); `place` comes with its leading space */
	phrase(subject: string, verb: string, place: string): string;
	/** "auf Slice 4" / "on slice 4" */
	onSlice(n: number): string;
	/** "in Szene x" / "in scene x" */
	inGroup(group: GroupRef): string;
	/** "Szene x" / "Scene x" (a prefix) */
	group(group: GroupRef): string;
	/** "Slice 4" */
	slice(n: number): string;
	/** "Neue Slices 40–44" (count 1: "Neuer Slice 40") */
	newSlices(range: string, count: number): string;
	/** "Slices 3–5 entfernt" */
	removedSlices(range: string, count: number): string;
	/** "Texte (en, fr) übersetzt" */
	translated(languages: string[]): string;
	/** the plural without a count: "Gruppen", "Collage-Stücke" (what "reordered" says) */
	plural(kind: NoteKind): string;
	/** a field's name ("Titel"); null = not one the note names (counted as settings) */
	field(key: string): string | null;
	/** "Einstellungen" */
	settings: string;
	/** "Neu angelegt" */
	created: string;
}

export const NOTE_MAX = 90;
const KIND_ORDER: NoteKind[] = ['created', 'textbox', 'item', 'piece', 'tail', 'slice', 'group', 'event', 'sound', 'text', 'settings', 'entry'];
const ACTION_ORDER: NoteAction[] = ['added', 'removed', 'moved', 'recoloured', 'set', 'changed', 'reordered', 'translated'];

// ── comparing ────────────────────────────────────────────────────────────────────────────────────────────────────
const isObj = (v: unknown): v is Record<string, any> => !!v && typeof v === 'object' && !Array.isArray(v);
const same = (a: unknown, b: unknown) => JSON.stringify(a ?? null) === JSON.stringify(b ?? null);
function isLocaleMap(v: unknown, L: NoteLanguages): boolean {
	if (!L || !isObj(v)) return false;
	const keys = Object.keys(v);
	return keys.length > 0 && keys.every((k) => L.languages.includes(k) && (v[k] === null || typeof v[k] === 'string' || Array.isArray(v[k])));
}
/** what differs: `structure` (anything but a translation) and the translations' languages */
interface Delta {
	structure: boolean;
	langs: Set<string>;
}
const delta = (): Delta => ({ structure: false, langs: new Set() });
function compare(a: unknown, b: unknown, L: NoteLanguages, out: Delta = delta()): Delta {
	if (same(a, b)) return out;
	if (isLocaleMap(a, L) || isLocaleMap(b, L)) {
		const ma = isObj(a) ? a : {};
		const mb = isObj(b) ? b : {};
		for (const k of new Set([...Object.keys(ma), ...Object.keys(mb)]))
			if (!same(ma[k], mb[k])) {
				if (L && k !== L.source && L.languages.includes(k)) out.langs.add(k);
				else out.structure = true;
			}
	} else if (Array.isArray(a) && Array.isArray(b) && a.length === b.length) a.forEach((x, i) => compare(x, b[i], L, out));
	else if (isObj(a) && isObj(b)) for (const k of new Set([...Object.keys(a), ...Object.keys(b)])) compare(a[k], b[k], L, out);
	else out.structure = true;
	return out;
}
const without = (o: Record<string, any> | undefined, keys: string[]) => Object.fromEntries(Object.entries(o ?? {}).filter(([k]) => !keys.includes(k)));
const list = (v: unknown): any[] => (Array.isArray(v) ? v : []);

/** pairs two lists by `key` (an id), the rest by position: [before, after] with undefined for a missing side */
function pairById(a: any[], b: any[], key: (x: any) => string | undefined): [any, any, number, number][] {
	const out: [any, any, number, number][] = [];
	const byKey = new Map<string, number>();
	b.forEach((x, j) => {
		const k = key(x);
		if (k !== undefined && !byKey.has(k)) byKey.set(k, j);
	});
	const used = new Set<number>();
	const loose: number[] = [];
	a.forEach((x, i) => {
		const k = key(x);
		const j = k !== undefined ? byKey.get(k) : undefined;
		if (j !== undefined && !used.has(j)) {
			used.add(j);
			out.push([x, b[j], i, j]);
		} else loose.push(i);
	});
	const free = b.map((_, j) => j).filter((j) => !used.has(j) && key(b[j]) === undefined);
	for (const i of loose) {
		const j = key(a[i]) === undefined ? free.shift() : undefined;
		if (j !== undefined) {
			used.add(j);
			out.push([a[i], b[j], i, j]);
		} else out.push([a[i], undefined, i, -1]);
	}
	b.forEach((x, j) => !used.has(j) && out.push([undefined, x, -1, j]));
	return out;
}
const idOf = (x: any) => (isObj(x) && typeof x.id === 'string' && x.id ? x.id : undefined);

// ── a chapter / a scene ─────────────────────────────────────────────────────────────────────────────────────────
function shortText(text: unknown, L: NoteLanguages): string | undefined {
	const value = isObj(text) ? (text[L?.source ?? ''] ?? Object.values(text).find((v) => v)) : text;
	const first = (Array.isArray(value) ? value.join(' ') : String(value ?? '')).replace(/\s+/g, ' ').trim();
	if (!first) return undefined;
	return first.length > 12 ? `${first.slice(0, 10).trimEnd()}…` : first;
}
const MOVE_KEYS = ['anchor', 'rotation'];
/** moved = only the place (x / y, anchor, rotation) differs */
function movedOnly(a: any, b: any, L: NoteLanguages, extra: string[] = []): 'moved' | 'changed' | null {
	const rest = (o: any) => ({ ...without(o, [...MOVE_KEYS, ...extra, 'rect']), rect: without(o?.rect, ['x', 'y']) });
	const place = (o: any) => ({ x: o?.rect?.x, y: o?.rect?.y, anchor: o?.anchor, rotation: o?.rotation });
	const r = compare(rest(a), rest(b), L).structure;
	if (r) return 'changed';
	return same(place(a), place(b)) ? null : 'moved';
}

function textboxes(a: any[], b: any[], place: NotePlace, L: NoteLanguages, out: NoteChange[], langs: Set<string>) {
	for (const [x, y] of pairById(a, b, idOf)) {
		if (!x) out.push({ kind: 'textbox', action: 'added', place, name: shortText(y.text, L) });
		else if (!y) out.push({ kind: 'textbox', action: 'removed', place, name: shortText(x.text, L) });
		else {
			for (const l of compare(x.text, y.text, L).langs) langs.add(l);
			const box = movedOnly({ ...x, text: textOf(x.text, L) }, { ...y, text: textOf(y.text, L) }, L, ['tail']);
			if (box) out.push({ kind: 'textbox', action: box, place, name: shortText(y.text, L) });
			if (!same(x.tail, y.tail)) out.push({ kind: 'tail', action: !y.tail || y.tail === 'none' ? 'removed' : 'set', place });
		}
	}
}
/** a text with its translations left out (they count as "translated") */
const textOf = (text: unknown, L: NoteLanguages) => (isLocaleMap(text, L) && L ? (text as Record<string, unknown>)[L.source] : text);

function placed(kind: 'item' | 'piece', a: any[], b: any[], place: NotePlace, ctx: NoteContext, out: NoteChange[]) {
	const name = (x: any) => (kind === 'item' ? (ctx.itemName?.(x.item) ?? x.item) : (ctx.pieceName?.(x) ?? pieceName(x)));
	const pairs = pairById(a, b, idOf);
	let changed = false;
	for (const [x, y] of pairs) {
		if (!x) out.push({ kind, action: 'added', place, name: name(y) });
		else if (!y) out.push({ kind, action: 'removed', place, name: name(x) });
		else {
			const look = kind === 'piece' && !same([x.preset, x.style], [y.preset, y.style]);
			const rest = movedOnly(x, y, ctx.languages, kind === 'piece' ? ['preset', 'style'] : []);
			const action = look ? (rest ? 'changed' : 'recoloured') : rest;
			if (action) out.push({ kind, action, place, name: name(y) });
			changed ||= !!action;
		}
	}
	// pieces are drawn in array order: the same pieces in another order = "reordered"
	const kept = pairs.filter(([x, y]) => x && y);
	if (!changed && kind === 'piece' && pairs.length === kept.length && kept.some(([, , , j], k) => k > 0 && j < kept[k - 1][3])) out.push({ kind, action: 'reordered', place });
}

function events(a: any[], b: any[], place: NotePlace, L: NoteLanguages, out: NoteChange[], langs: Set<string>) {
	for (const [x, y] of pairById(a, b, idOf)) {
		if (!x) out.push({ kind: 'event', action: 'added', place });
		else if (!y) out.push({ kind: 'event', action: 'removed', place });
		else {
			const d = compare(x, y, L);
			d.langs.forEach((l) => langs.add(l));
			if (d.structure) out.push({ kind: 'event', action: 'changed', place });
		}
	}
}

const SLICE_PARTS = ['dialog', 'items', 'pieces', 'events', 'audio'];
function slice(a: any, b: any, place: NotePlace, ctx: NoteContext, out: NoteChange[], langs: Set<string>) {
	const L = ctx.languages;
	const own = compare(without(a, SLICE_PARTS), without(b, SLICE_PARTS), L);
	own.langs.forEach((l) => langs.add(l));
	if (own.structure) out.push({ kind: 'slice', action: 'changed', place });
	textboxes(list(a.dialog), list(b.dialog), place, L, out, langs);
	placed('item', list(a.items), list(b.items), place, ctx, out);
	placed('piece', list(a.pieces), list(b.pieces), place, ctx, out);
	events(list(a.events), list(b.events), place, L, out, langs);
	if (!same(a.audio, b.audio)) out.push({ kind: 'sound', action: 'changed', place });
}

/** slices pair by their picture (a longest common run), then a moved picture, then by position (a picture replaced) */
function slices(a: any[], b: any[], group: GroupRef | undefined, ctx: NoteContext, out: NoteChange[], langs: Set<string>) {
	const img = (s: any) => String(s?.image ?? '');
	const n = a.length;
	const m = b.length;
	const lcs = Array.from({ length: n + 1 }, () => new Array<number>(m + 1).fill(0));
	for (let i = n - 1; i >= 0; i--) for (let j = m - 1; j >= 0; j--) lcs[i][j] = img(a[i]) && img(a[i]) === img(b[j]) ? lcs[i + 1][j + 1] + 1 : Math.max(lcs[i + 1][j], lcs[i][j + 1]);
	const pairs: [number, number][] = [];
	const left: number[] = [];
	const right: number[] = [];
	for (let i = 0, j = 0; i < n || j < m; ) {
		if (i < n && j < m && img(a[i]) && img(a[i]) === img(b[j])) pairs.push([i++, j++]);
		else if (j < m && (i >= n || lcs[i][j + 1] >= lcs[i + 1][j])) right.push(j++);
		else left.push(i++);
	}
	for (const j of [...right]) {
		const i = left.find((k) => img(a[k]) && img(a[k]) === img(b[j]));
		if (i === undefined) continue;
		left.splice(left.indexOf(i), 1);
		right.splice(right.indexOf(j), 1);
		out.push({ kind: 'slice', action: 'moved', place: { group, slice: j + 1 } });
		pairs.push([i, j]);
	}
	while (left.length && right.length) pairs.push([left.shift()!, right.shift()!]);
	for (const [i, j] of pairs) slice(a[i], b[j], { group, slice: j + 1 }, ctx, out, langs);
	for (const j of right) out.push({ kind: 'slice', action: 'added', place: { group, slice: j + 1 } });
	for (const i of left) out.push({ kind: 'slice', action: 'removed', place: { group, slice: i + 1 } });
}

function fields(a: any, b: any, skip: string[], place: NotePlace | undefined, L: NoteLanguages, out: NoteChange[], langs: Set<string>) {
	for (const key of new Set([...Object.keys(a ?? {}), ...Object.keys(b ?? {})])) {
		if (skip.includes(key)) continue;
		const d = compare(a?.[key], b?.[key], L);
		d.langs.forEach((l) => langs.add(l));
		if (d.structure) out.push(key === 'audio' ? { kind: 'sound', action: 'changed', place } : { kind: 'settings', action: 'changed', place, field: key });
	}
}
const translated = (langs: Set<string>, out: NoteChange[]) => [...langs].forEach((lang) => out.push({ kind: 'text', action: 'translated', lang }));

/** a group's (a scene document's) changes; `ref` names it in the note */
export function groupChanges(before: any, after: any, ref: GroupRef | undefined, ctx: NoteContext, langs = new Set<string>(), top = true): NoteChange[] {
	const out: NoteChange[] = [];
	const place = ref ? { group: ref } : undefined;
	fields(before, after, ['slices', 'events'], place, ctx.languages, out, langs);
	events(list(before?.events), list(after?.events), place ?? {}, ctx.languages, out, langs);
	slices(list(before?.slices), list(after?.slices), ref, ctx, out, langs);
	if (top) translated(langs, out);
	return out;
}

/** the group's name for the note: a scene by its file's name, else its label, else its number */
export function groupRef(group: any, index: number): GroupRef {
	const scene = /([^/]+)\.json$/.exec(String(group?.ref?.src ?? ''))?.[1];
	if (scene) return { scene };
	return typeof group?.label === 'string' && group.label.trim() ? { label: group.label.trim() } : { number: index + 1 };
}

/** a chapter's changes (its own groups; a scene's content is its own document – `groupChanges`) */
export function chapterChanges(before: any, after: any, ctx: NoteContext): NoteChange[] {
	if (!before) return [{ kind: 'created', action: 'added' }];
	const out: NoteChange[] = [];
	const langs = new Set<string>();
	fields(before, after, ['groups'], undefined, ctx.languages, out, langs);
	const a = list(before.groups);
	const b = list(after?.groups);
	const key = (g: any) => (g?.ref?.src ? `ref:${g.ref.src}` : typeof g?.label === 'string' && g.label ? `label:${g.label}` : undefined);
	const pairs = pairById(a, b, key);
	for (const [x, y, i, j] of pairs) {
		if (!x) out.push({ kind: 'group', action: 'added', name: nameOfGroup(groupRef(y, j)) });
		else if (!y) out.push({ kind: 'group', action: 'removed', name: nameOfGroup(groupRef(x, i)) });
		else out.push(...groupChanges(x, y, groupRef(y, j), ctx, langs, false));
	}
	const kept = pairs.filter(([x, y]) => x && y);
	if (kept.some(([, , , j], k) => k > 0 && j < kept[k - 1][3])) out.push({ kind: 'group', action: 'reordered' });
	translated(langs, out);
	return out;
}
const nameOfGroup = (g: GroupRef) => ('scene' in g ? g.scene : 'label' in g ? g.label : undefined);

// ── a meta page (cast, credits, toon settings, the catalog …) ─────────────────────────────────────────────────────
const ENTRY_KEYS = ['id', 'character', 'key', 'slug', 'number', 'name'];
function entryKey(items: any[]): ((x: any) => string | undefined) | null {
	const k = ENTRY_KEYS.find((key) => items.length > 0 && items.every((x) => isObj(x) && (typeof x[key] === 'string' || typeof x[key] === 'number')));
	return k ? (x) => (isObj(x) ? String(x[k]) : undefined) : null;
}
function entryName(x: any, L: NoteLanguages): string | undefined {
	for (const k of ['name', 'title', 'label']) if (x?.[k]) return shortName(x[k], L);
	for (const k of ENTRY_KEYS) if (typeof x?.[k] === 'string' || typeof x?.[k] === 'number') return String(x[k]);
	return undefined;
}
const shortName = (v: unknown, L: NoteLanguages) => {
	const s = isObj(v) ? (v[L?.source ?? ''] ?? Object.values(v).find((x) => x)) : v;
	return typeof s === 'string' ? s : undefined;
};
function entries(a: any[], b: any[], L: NoteLanguages, out: NoteChange[], langs: Set<string>) {
	const key = entryKey([...a, ...b]);
	if (!key) {
		const d = compare(a, b, L);
		d.langs.forEach((l) => langs.add(l));
		if (d.structure) out.push({ kind: 'entry', action: 'changed' });
		return;
	}
	const pairs = pairById(a, b, key);
	const start = out.length;
	for (const [x, y] of pairs) {
		if (!x) out.push({ kind: 'entry', action: 'added', name: entryName(y, L) });
		else if (!y) out.push({ kind: 'entry', action: 'removed', name: entryName(x, L) });
		else {
			const d = compare(x, y, L);
			d.langs.forEach((l) => langs.add(l));
			if (d.structure) out.push({ kind: 'entry', action: 'changed', name: entryName(y, L) });
		}
	}
	const kept = pairs.filter(([x, y]) => x && y);
	if (out.length === start && kept.some(([, , , j], k) => k > 0 && j < kept[k - 1][3])) out.push({ kind: 'entry', action: 'reordered' });
}
/** an object of lists (the cast's `seasons`: { "1": [...] }) is walked like the document's own lists */
const listHolder = (v: unknown) => isObj(v) && !Array.isArray(v) && Object.values(v).some(Array.isArray);

/** a meta document's changes: its lists by entry (added / removed / changed, named), its other fields by name */
export function metaChanges(before: any, after: any, ctx: NoteContext): NoteChange[] {
	if (before === null || before === undefined) return [{ kind: 'created', action: 'added' }];
	const L = ctx.languages;
	const out: NoteChange[] = [];
	const langs = new Set<string>();
	if (Array.isArray(before) || Array.isArray(after)) entries(list(before), list(after), L, out, langs);
	else
		for (const key of new Set([...Object.keys(before ?? {}), ...Object.keys(after ?? {})])) {
			const x = before?.[key];
			const y = after?.[key];
			if (Array.isArray(x) || Array.isArray(y)) entries(list(x), list(y), L, out, langs);
			else if (listHolder(x) || listHolder(y)) for (const k of new Set([...Object.keys(x ?? {}), ...Object.keys(y ?? {})])) entries(list(x?.[k]), list(y?.[k]), L, out, langs);
			else {
				const d = compare(x, y, L);
				d.langs.forEach((l) => langs.add(l));
				if (d.structure) out.push({ kind: 'settings', action: 'changed', field: key });
			}
		}
	translated(langs, out);
	return out;
}

// ── the line ────────────────────────────────────────────────────────────────────────────────────────────────────
const groupKey = (g?: GroupRef) => (g ? JSON.stringify(g) : '');
const placeKey = (p?: NotePlace) => `${groupKey(p?.group)}#${p?.slice ?? ''}`;
const capital = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);
/** "3, 7, 40–44" */
export function numberRange(numbers: number[]): string {
	const sorted = [...new Set(numbers)].sort((a, b) => a - b);
	const runs: string[] = [];
	for (let i = 0; i < sorted.length; ) {
		let j = i;
		while (j + 1 < sorted.length && sorted[j + 1] === sorted[j] + 1) j++;
		runs.push(j > i ? `${sorted[i]}–${sorted[j]}` : String(sorted[i]));
		i = j + 1;
	}
	return runs.join(', ');
}

/**
 * The note: one phrase per kind + action ("Textbox 'Na, servus…' verschoben", "3 Textboxen in Szene x geändert"), the
 * place named when the phrase has one, a shared place once in front ("Slice 12: …"), at most `max` characters (whole
 * phrases, then "…"). `multipleGroups`: the page edits more than one group – a slice's place names its group too.
 */
export function summarizeChanges(changes: NoteChange[], words: NoteWords, options: { multipleGroups?: boolean; max?: number } = {}): string {
	const max = options.max ?? NOTE_MAX;
	const multi = !!options.multipleGroups;
	const sorted = [...changes].sort((a, b) => KIND_ORDER.indexOf(a.kind) - KIND_ORDER.indexOf(b.kind) || ACTION_ORDER.indexOf(a.action) - ACTION_ORDER.indexOf(b.action));
	const buckets = new Map<string, NoteChange[]>();
	for (const c of sorted) buckets.set(`${c.kind}/${c.action}`, [...(buckets.get(`${c.kind}/${c.action}`) ?? []), c]);
	if (!buckets.size) return '';
	// a place shared by every phrase goes in front once ("Slice 12: …", "Szene x: …") – translations have none
	const placed = sorted.filter((c) => c.kind !== 'text');
	const first = placed[0]?.place;
	const oneSlice = !!first?.slice && placed.every((c) => placeKey(c.place) === placeKey(first));
	const oneGroup = multi && !!first?.group && placed.every((c) => groupKey(c.place?.group) === groupKey(first.group));
	const front = buckets.size > 1 ? (oneSlice ? 'slice' : oneGroup ? 'group' : null) : null;
	/** a phrase's place: its slice when all of it is on one, its group when the page has several and all of it is in one */
	const where = (list: NoteChange[], withSlice = true) => {
		if (front === 'slice') return '';
		const p = list[0].place;
		const slice = withSlice && p?.slice && list.every((c) => placeKey(c.place) === placeKey(p)) ? ` ${words.onSlice(p.slice)}` : '';
		const g = p?.group;
		const group = g && multi && front !== 'group' && list.every((c) => groupKey(c.place?.group) === groupKey(g)) ? ` ${words.inGroup(g)}` : '';
		return slice + group;
	};
	const phrases: string[] = [];
	for (const list of buckets.values()) {
		const { kind, action } = list[0];
		if (kind === 'created') phrases.push(words.created);
		else if (kind === 'text') phrases.push(words.translated([...new Set(list.map((c) => c.lang ?? ''))]));
		else if (action === 'reordered') phrases.push(words.phrase(words.plural(kind), words.verb(action, kind), where(list)));
		else if (kind === 'slice' && (action === 'added' || action === 'removed')) {
			const range = numberRange(list.map((c) => c.place?.slice ?? 0));
			phrases.push((action === 'added' ? words.newSlices(range, list.length) : words.removedSlices(range, list.length)) + where(list, false));
		} else if (kind === 'slice' && list.length === 1 && front !== 'slice') phrases.push(words.phrase(words.slice(list[0].place?.slice ?? 0), words.verb(action, kind), where(list, false)));
		else if (kind === 'settings') {
			const names = [...new Set(list.map((c) => c.field ?? ''))].map((f) => words.field(f));
			const subject = names.length <= 3 && names.every((n) => n) ? names.join(', ') : words.settings;
			phrases.push(words.phrase(subject, words.verb(action, kind), where(list)));
		} else {
			const name = list.length === 1 ? list[0].name : undefined;
			phrases.push(words.phrase(words.subject(kind, list.length) + (name ? ` '${name}'` : ''), words.verb(action, kind), where(list)));
		}
	}
	let prefix = '';
	if (front === 'slice') prefix = `${multi && first?.group ? `${capital(words.group(first.group))}, ` : ''}${words.slice(first!.slice!)}: `;
	else if (front === 'group') prefix = `${capital(words.group(first!.group!))}: `;
	const join = (list: string[]) => prefix + list.map((p, i) => (i || prefix ? p : capital(p))).join(', ');
	const full = join(phrases);
	if (full.length <= max) return full;
	// whole phrases while they fit, then "…"; a first phrase too long on its own is cut
	for (let n = phrases.length - 1; n > 0; n--) {
		const line = `${join(phrases.slice(0, n))}, …`;
		if (line.length <= max) return line;
	}
	return `${full.slice(0, max - 1).trimEnd()}…`;
}
