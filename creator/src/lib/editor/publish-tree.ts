/**
 * Publish down the tree (BACKLOG 10.60, owner 2026-10-11: "über jede höhere Hierarchie kann man das Zeug darunter
 * publishen – mit einer Liste, mit Checkboxen an- und abwählen; zusammenfassende Änderungen, aber trotzdem granular"):
 * every stored file below a level that has an unpublished draft, as a checklist (all ticked); the ticked ones are
 * published – or submitted for review where the account's review rules ask for it.
 *
 * The world map's places and routes and the registry's characters are files of their own (`place:<id>`, `route:<id>`,
 * `character:<id>` – agents/MAP-DOCUMENTS.md; owner 2026-10-12): one line each, by its name, "new" / "changed" /
 * "removed" – so no document is published in part any more. The ticked files go live in one transaction
 * (POST /entities/publish): a route and the new place it runs through are never half published.
 *
 * Levels: everything (the platform: every toon + the world – map, characters, settings …) → a toon (all its files) → a
 * season (itself, its chapters and their scene files) → a chapter (itself and its scene files). A toon, a season and a
 * chapter also list the platform-wide files their pieces draw with (`PLATFORM_ALONG`: the custom collage presets –
 * research/COLLAGE.md §12.4), so no level ships pieces whose preset is still a draft.
 *
 * Catalog entries (`item:<id>` …, agents/CATALOG-IN-CREATOR.md) are files of the world level (no toon in their key):
 * the server decides who publishes them (their toon is the entry's; a physical change needs the `physical` rule) –
 * a 403 review_required puts the line into the submit for review instead.
 */
import { checklistDialog, askDialog, type ChecklistGroup } from '@scutoons/ui';
import { ApiError, catalogDocKey, type ContentIndexEntity } from '@scutoons/api';
import { contentApi, contentIndex, publishMany, withContent } from '$lib/api.ts';
import { fileLabel } from '$lib/labels.ts';
import { reviews } from '$lib/editor/reviews.svelte.ts';
import { m } from '$lib/paraglide/messages.js';
import { getLocale } from '$lib/paraglide/runtime.js';

export type PublishScope =
	| { kind: 'all'; toons: { id: string; name: string }[] }
	/** a page without levels below (the map, characters, cast, credits, the script …): its own files – the same list */
	| { kind: 'files'; toon: string; keys: string[] }
	| { kind: 'toon'; toon: string }
	| { kind: 'season'; toon: string; number: number; chapters: number[] }
	| { kind: 'chapter'; toon: string; number: number };

/** the platform-wide files a toon's, a season's and a chapter's list offer too (their pieces name them) */
export const PLATFORM_ALONG: readonly string[] = ['colorations'];
/** the levels that list them */
const alongWith = (scope: PublishScope): scope is Extract<PublishScope, { kind: 'toon' | 'season' | 'chapter' }> =>
	scope.kind === 'toon' || scope.kind === 'season' || scope.kind === 'chapter';

/** a stored file with a draft newer than what readers see – a newer version that equals the published one is none (owner D6) */
const hasDraft = (e: ContentIndexEntity) => (e.published === null || e.latest > e.published) && !e.same_as_published;

/** is a key below the level (or the level's own file)? */
export function inScope(key: string, scope: PublishScope): boolean {
	if (alongWith(scope) && PLATFORM_ALONG.includes(key)) return true;
	const chapter = (t: string, n: number) => key === `chapter:${t}:${n}` || key.startsWith(`scene:${t}:${n}:`);
	switch (scope.kind) {
		case 'all':
			return true;
		case 'files':
			return scope.keys.includes(key);
		case 'toon':
			return !catalogDocKey(key) && (key.split(':')[1] === scope.toon || key === `toon:${scope.toon}`);
		case 'season':
			return key === `season:${scope.toon}:${scope.number}` || scope.chapters.some((n) => chapter(scope.toon, n));
		case 'chapter':
			return chapter(scope.toon, scope.number);
	}
}

/**
 * the order in a list: the level's own files first, then down the tree; the catalog last, in the order its entries
 * refer to each other (an item names its edition / album, a product its item, a recipe its items)
 */
const KIND_ORDER = ['toon', 'entry', 'season', 'chapter', 'scene', 'script', 'cast', 'credits', 'map', 'place', 'route', 'region', 'characters', 'character', 'colorations', 'edition', 'album', 'item', 'product', 'recipe'];
const rank = (key: string) => {
	const i = KIND_ORDER.indexOf(key.split(':')[0]);
	return i < 0 ? KIND_ORDER.length : i;
};
const byTree = (a: ContentIndexEntity, b: ContentIndexEntity) => rank(a.key) - rank(b.key) || a.key.localeCompare(b.key, undefined, { numeric: true });

/** a line's label: a scene names its chapter, a per-language file its language */
function label(key: string): string {
	const parts = key.split(':');
	if (parts[0] === 'scene' && parts.length >= 4) return m['creator.entity_group']({ number: parts[2], label: parts[3] });
	const base = fileLabel(key);
	return parts[0] === 'script' && parts[2] ? `${base} (${parts[2]})` : base;
}

/** a line's detail: the version, when, who and the note */
function detail(e: ContentIndexEntity): string {
	const when = e.latest_at ? new Date(e.latest_at).toLocaleString(getLocale(), { dateStyle: 'short', timeStyle: 'short' }) : '';
	return [`v${e.latest}`, when, e.latest_author ?? '', e.latest_message ? `„${e.latest_message}“` : ''].filter(Boolean).join(' · ');
}

/** the files with a draft at a level */
export async function pendingFiles(scope: PublishScope): Promise<ContentIndexEntity[]> {
	const toons = scope.kind === 'all' ? ['', ...scope.toons.map((t) => t.id)] : [scope.toon];
	// a toon's index holds only its own rows: `PLATFORM_ALONG` from the platform's – those keys alone (a platform key like
	// `character:catpope` would pass a toon's test)
	const along = alongWith(scope) && scope.toon ? contentIndex('').then((r) => r.entities.filter((e) => PLATFORM_ALONG.includes(e.key))) : [];
	const lists = await Promise.all([...toons.map((t) => contentIndex(t).then((r) => r.entities)), along]);
	return lists.flat().filter(hasDraft).filter((e) => inScope(e.key, scope)).sort(byTree);
}

/** one line of the checklist: one stored file (an entry of the map / the registry by its name) */
interface Line {
	id: string;
	file: ContentIndexEntity;
	/** an entry of a collection (a place, a route, a character – agents/MAP-DOCUMENTS.md): its name and what the draft does */
	part?: { kind: string; name: unknown; change: 'added' | 'changed' | 'removed' };
}

/** an entry's name in the creator's language (a locale map or a string), else its id */
function nameOf(name: unknown, id: string | null): string {
	if (typeof name === 'string' && name) return name;
	if (name && typeof name === 'object') {
		const map = name as Record<string, unknown>;
		const v = map[getLocale()] ?? map.de ?? Object.values(map)[0];
		if (typeof v === 'string' && v) return v;
	}
	return id ?? '';
}
/** the kinds stored one document per entry (a line each, by name) */
const PART_WORD: Record<string, () => string> = {
	place: () => m['creator.publish_part_place'](),
	route: () => m['creator.publish_part_route'](),
	region: () => m['creator.publish_part_region'](),
	character: () => m['creator.publish_part_character']()
};
const CHANGE_WORD: Record<'added' | 'changed' | 'removed', () => string> = {
	added: () => m['creator.publish_part_added'](),
	changed: () => m['creator.publish_part_changed'](),
	removed: () => m['creator.publish_part_removed']()
};
const lineLabel = (l: Line) => (l.part ? `${PART_WORD[l.part.kind]()}: ${nameOf(l.part.name, l.file.key.split(':')[1] ?? null)}` : label(l.file.key));
const lineDetail = (l: Line) => (l.part ? `${CHANGE_WORD[l.part.change]()} · ${detail(l.file)}` : detail(l.file));

/** The lines for the files: one per file; an entry of the map / the registry by its name (one request for their names) */
async function linesOf(files: ContentIndexEntity[]): Promise<Line[]> {
	const kinds = [...new Set(files.map((f) => f.kind).filter((k) => k in PART_WORD))];
	const entries = kinds.length ? new Map((await withContent<{ name?: unknown; removed?: boolean }>(kinds, 'draft')).entities.map((e) => [e.key, e])) : null;
	return files.map((file) => {
		const entry = entries?.get(file.key);
		if (!entry) return { id: file.key, file };
		const change = entry.content?.removed ? 'removed' : file.published === null ? 'added' : 'changed';
		return { id: file.key, file, part: { kind: file.kind, name: entry.content?.name, change } };
	});
}

/**
 * The checklist for a level, then publish / submit the ticked lines. Returns the message for the page (or null when
 * cancelled) and the keys that went live (whole or in part).
 */
export async function publishTree(scope: PublishScope, title: string): Promise<{ message: string; published: string[] } | null> {
	const files = await pendingFiles(scope);
	if (!files.length) return { message: m['creator.nothing_to_publish'](), published: [] };
	const lines = await linesOf(files);
	const toonName = (id: string | null) => (id ? (scope.kind === 'all' ? scope.toons.find((t) => t.id === id)?.name : undefined) ?? id : m['creator.publish_tree_world']());
	const item = (l: Line) => ({ id: l.id, label: lineLabel(l), detail: lineDetail(l) });
	const groups: ChecklistGroup[] =
		scope.kind === 'all'
			? [...new Set(lines.map((l) => l.file.toon ?? ''))].map((t) => ({ label: toonName(t || null), items: lines.filter((l) => (l.file.toon ?? '') === t).map(item) }))
			: [{ items: lines.map(item) }];
	const review = files.some((f) => reviews.needsReview(f.toon));
	const picked = await checklistDialog({
		title,
		message: m['creator.publish_tree_message'](),
		groups,
		countLabel: (n) => (review ? m['creator.publish_tree_submit']({ count: n }) : m['creator.publish_tree_publish']({ count: n })),
		allLabel: m['creator.publish_tree_all'](),
		noneLabel: m['creator.publish_tree_none'](),
		cancelLabel: m['creator.cancel']()
	});
	if (!picked?.length) return null;
	const chosen = new Set(picked);
	const done: string[] = [];
	const forReview: { key: string; version: number }[] = [];
	// the ticked files go live together, in one transaction (a route and the new place it runs through never half);
	// the version the list showed – a newer save in between is not published unseen
	let live: { key: string; version: number }[] = [];
	for (const file of files) {
		if (!chosen.has(file.key)) continue;
		if (reviews.needsReview(file.toon)) forReview.push({ key: file.key, version: file.latest });
		else live.push({ key: file.key, version: file.latest });
	}
	// the catalog entries too (publishing writes the catalog table): the server's rules decide (a catalog entry's toon,
	// a physical change – Reviews) and name the keys that need a review – nothing went out; those are submitted, the
	// rest goes live together
	while (live.length) {
		try {
			await publishMany(live);
			done.push(...live.map((w) => w.key));
			break;
		} catch (e) {
			const keys: unknown = e instanceof ApiError && e.code === 'review_required' ? e.body?.keys : null;
			if (!Array.isArray(keys) || !keys.length) throw e;
			forReview.push(...live.filter((w) => keys.includes(w.key)));
			live = live.filter((w) => !keys.includes(w.key));
		}
	}
	let submitted = '';
	if (forReview.length) {
		const note = await askDialog({ title: m['creator.submit_title'](), label: m['creator.submit_note_label'](), placeholder: m['creator.submit_note_placeholder'](), confirmLabel: m['creator.submit_send'](), skipLabel: m['creator.submit_send']() });
		if (note !== null) {
			const result = await contentApi.submitReview(forReview, note.trim());
			void reviews.load();
			submitted = m['creator.submitted']({ reviewers: result.reviewers.join(', ') });
		}
	}
	const message = [done.length ? m['creator.publish_tree_done']({ count: done.length }) : '', submitted].filter(Boolean).join(' ');
	return { message: message || m['creator.nothing_to_publish'](), published: done };
}
