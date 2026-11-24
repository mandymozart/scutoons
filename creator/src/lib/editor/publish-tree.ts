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
 * season (itself and its chapters) → a chapter (itself). A toon, a season and a
 * chapter also list the platform-wide files their pieces draw with (`PLATFORM_ALONG`: the custom collage presets –
 * research/COLLAGE.md §12.4), so no level ships pieces whose preset is still a draft; a score's pages (its catalog entry,
 * its collage stage) name them among their own files.
 *
 * Catalog entries (`item:<id>` …, agents/CATALOG-IN-CREATOR.md) are files of the world level (no toon in their key):
 * the server decides who publishes them (their toon is the entry's; a physical change needs the `physical` rule) –
 * a 403 review_required puts the line into the submit for review instead.
 *
 * The ticked files go to the server in one request (`partial`): it publishes them in dependency order (modules →
 * instruments → presets → rigs → … → the content), each unit on its own, so one refusal blocks only what depends on it;
 * the answer names what went live and every refusal with its reason – the Publishes lines of the Log. A factory module
 * (`read_only` in the index) is never a line: it is built in the catalog.
 *
 * Deck scenes (`scene:<id>`) are lines too (owner 2026-11-22): the partial publish takes them last, through the core's
 * scene check (as their own page publishes them), after the modules, instruments, presets and rigs they play.
 *
 * Every Publish of the creator is this one (owner 2026-11-22: "the same publish algo everywhere"), and it has one way in
 * (RULES H18): the bar's Publish opens its dialog (publish-flow.svelte.ts) on everything – the scope's drafts plus the
 * unpublished drafts they depend on, transitively, which the server names (`POST /entities/publish/plan`, the one
 * dependency rule: PublishOrder::refs); those lines say what needs them.
 */
import { catalogDocKey, type ContentIndexEntity } from '@scutoons/api';
import { contentApi, contentIndex, publishPartial, publishPlan, withContent } from '$lib/api.ts';
import { publishNotices, type PublishNotice, type PublishRefusal } from '$lib/editor/publish-report.ts';
import { parseEntityKey } from '@scutoons/content';
import { fileLabel, kindWord } from '$lib/labels.ts';
import { reviews } from '$lib/editor/reviews.svelte.ts';
import { toast } from '$lib/toasts.ts';
import { m } from '$lib/paraglide/messages.js';
import { getLocale } from '$lib/paraglide/runtime.js';

export type PublishScope =
	| { kind: 'all'; toons: { id: string; name: string }[] }
	/** a page without levels below (the map, characters, cast, credits, the script …): its own files – the same list */
	| { kind: 'files'; toon: string; keys: string[] }
	| { kind: 'toon'; toon: string }
	| { kind: 'season'; toon: string; number: number; chapters: number[] }
	| { kind: 'chapter'; toon: string; number: number }
	/** one file at a chosen version (a step of its history) */
	| { kind: 'version'; toon: string; key: string; version: number };

/** the platform-wide files a toon's, a season's and a chapter's list offer too (their pieces name them) */
export const PLATFORM_ALONG: readonly string[] = ['colorations'];
/** the levels that list them */
const alongWith = (scope: PublishScope): scope is Extract<PublishScope, { kind: 'toon' | 'season' | 'chapter' }> =>
	scope.kind === 'toon' || scope.kind === 'season' || scope.kind === 'chapter';

/** a stored file with a draft newer than what readers see – a newer version that equals the published one is none (owner D6) */
const hasDraft = (e: ContentIndexEntity) => (e.published === null || e.latest > e.published) && !e.same_as_published && !e.read_only;

/** is a key below the level (or the level's own file)? */
export function inScope(key: string, scope: PublishScope): boolean {
	if (alongWith(scope) && PLATFORM_ALONG.includes(key)) return true;
	const chapter = (t: string, n: number) => key === `chapter:${t}:${n}`;
	switch (scope.kind) {
		case 'all':
			return true;
		case 'files':
			return scope.keys.includes(key);
		case 'version':
			return key === scope.key;
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
const KIND_ORDER = ['toon', 'entry', 'season', 'chapter', 'season-script', 'chapter-script', 'kanon', 'cast', 'credits', 'map', 'place', 'route', 'region', 'characters', 'character', 'colorations', 'show', 'module', 'instrument', 'preset', 'rig', 'device-preset', 'rig-preset', 'edition', 'album', 'item', 'product', 'recipe', 'home-scene', 'airlift-carrier'];
const rank = (key: string) => {
	const i = KIND_ORDER.indexOf(key.split(':')[0]);
	return i < 0 ? KIND_ORDER.length : i;
};
const byTree = (a: ContentIndexEntity, b: ContentIndexEntity) => rank(a.key) - rank(b.key) || a.key.localeCompare(b.key, undefined, { numeric: true });

/** a line's label: a per-language file names its language */
function label(key: string): string {
	const ref = parseEntityKey(key);
	const base = fileLabel(key);
	return ref?.locale ? `${base} (${ref.locale})` : base;
}

/** a line's detail: the version, when, who and the note */
function detail(e: ContentIndexEntity): string {
	const when = e.latest_at ? new Date(e.latest_at).toLocaleString(getLocale(), { dateStyle: 'short', timeStyle: 'short' }) : '';
	return [`v${e.latest}`, when, e.latest_author ?? '', e.latest_message ? `„${e.latest_message}“` : ''].filter(Boolean).join(' · ');
}

/** the files with a draft at a level (the deck scenes too – the partial publish takes them last) */
export async function pendingFiles(scope: PublishScope): Promise<ContentIndexEntity[]> {
	if (scope.kind === 'version') {
		const row = (await contentIndex(scope.toon)).entities.find((e) => e.key === scope.key);
		return row ? [{ ...row, latest: scope.version }] : [];
	}
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
	/** a draft dependency the server added: the key that needs it */
	neededBy?: string;
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
const lineDetail = (l: Line) => {
	const base = l.part ? `${CHANGE_WORD[l.part.change]()} · ${detail(l.file)}` : detail(l.file);
	return l.neededBy ? `${m['creator.publish_tree_needed_by']({ name: label(l.neededBy) })} · ${base}` : base;
};

/** The lines for the files: one per file; an entry of the map / the registry by its name (one request for their names) */
async function linesOf(files: ContentIndexEntity[], neededBy: Map<string, string>): Promise<Line[]> {
	const kinds = [...new Set(files.map((f) => f.kind).filter((k) => k in PART_WORD))];
	const entries = kinds.length ? new Map((await withContent<{ name?: unknown; removed?: boolean }>(kinds, 'draft')).entities.map((e) => [e.key, e])) : null;
	return files.map((file) => {
		const entry = entries?.get(file.key);
		const needs = neededBy.get(file.key);
		const line: Line = { id: file.key, file, ...(needs ? { neededBy: needs } : {}) };
		if (!entry) return line;
		const change = entry.content?.removed ? 'removed' : file.published === null ? 'added' : 'changed';
		return { ...line, part: { kind: file.kind, name: entry.content?.name, change } };
	});
}

export interface PublishTreeResult {
	/** the first Log line's text */
	message: string;
	/** the keys that went live (whole or in part) */
	published: string[];
	refused: PublishRefusal[];
	/** the Publishes lines: what went live by name, one per refusal with its reason */
	notices: PublishNotice[];
}


/**
 * The one collector: the scope's drafts and the unpublished drafts they depend on, transitively (the server's rule) –
 * each dependency with the key that needs it
 */
export async function collectPublishable(scope: PublishScope): Promise<{ files: ContentIndexEntity[]; neededBy: Map<string, string>; refused: Map<string, string> }> {
	const own = await pendingFiles(scope);
	if (!own.length) return { files: [], neededBy: new Map(), refused: new Map() };
	const have = new Set(own.map((f) => f.key));
	const asked = own.map((f) => ({ key: f.key, version: f.latest }));
	// the API takes 500 at a time
	const chunks = Array.from({ length: Math.ceil(asked.length / 500) }, (_, i) => asked.slice(i * 500, i * 500 + 500));
	const plans = await Promise.all(chunks.map((c) => publishPlan(c)));
	const found = plans.flatMap((r) => r.entities);
	const deps = [...new Map(found.filter((e) => !have.has(e.key) && hasDraft(e)).map((e) => [e.key, e])).values()];
	const files = [...own, ...deps].sort(byTree);
	// a key the server does not read: its line stays, with the server's reason, and is not sent
	const refused = new Map(plans.flatMap((r) => r.refused ?? []).map((r) => [r.key, r.reason]));
	return { files, neededBy: new Map(deps.map((d) => [d.key, d.needed_by])), refused };
}

/** one line of the Publish dialog (lap 2 A10): a file by its name, what its draft does, its kind; the rest is hover text */
export interface PublishLine {
	id: string;
	label: string;
	/** the kind's word (and the toon, in the list of everything) */
	type: string;
	/** what the draft does: new · changed · removed (a submit decided here: in review) */
	chip: string;
	/** version, when, who, the note – and the key that needs it, for a dependency */
	hint: string;
	/** this account submits it for review instead of publishing it */
	review: boolean;
	/** someone else's submit this account decides on: its reviews, published as submitted */
	decide?: number[];
	/** the server refuses this key (the plan's reason): shown on the line, never ticked */
	refused?: string;
}

/** what a scope has to publish: the files and their lines, in the tree's order */
export interface Publishable {
	files: ContentIndexEntity[];
	lines: PublishLine[];
}

/** The dialog's list for a scope: its drafts and what they need, one line each */
export async function publishable(scope: PublishScope): Promise<Publishable> {
	const { files, neededBy, refused } = await collectPublishable(scope);
	if (!files.length) return { files, lines: [] };
	const toonName = (id: string | null) => (id ? ((scope.kind === 'all' ? scope.toons.find((t) => t.id === id)?.name : undefined) ?? id) : '');
	const lines = (await linesOf(files, neededBy)).map((l) => ({
		id: l.id,
		...(refused.has(l.id) ? { refused: refused.get(l.id) } : {}),
		label: l.part ? nameOf(l.part.name, l.file.key.split(':')[1] ?? null) : label(l.file.key),
		type: [kindWord(l.file.key) ?? '', scope.kind === 'all' ? toonName(l.file.toon) : ''].filter(Boolean).join(' · '),
		chip: CHANGE_WORD[l.part?.change ?? (l.file.published === null ? 'added' : 'changed')](),
		hint: lineDetail(l),
		review: reviews.needsReview(l.file.toon)
	}));
	return { files, lines };
}

/**
 * Publish / submit the ticked files: they go to the server in one ordered partial publish; what the account's review
 * rules (or the server's answer) send to review is submitted with the note `askNote` returns (null: not submitted).
 * `decided`: the keys a reviewer's decision put live just before, named with the rest. The Publishes lines go to the Log
 * as well.
 */
export async function publishChosen(files: ContentIndexEntity[], chosen: ReadonlySet<string>, askNote: () => Promise<string | null>, decided: string[] = []): Promise<PublishTreeResult> {
	const done: string[] = [...decided];
	const refused: PublishRefusal[] = [];
	const forReview: { key: string; version: number }[] = [];
	// the version the list showed – a newer save in between is not published unseen
	const live: { key: string; version: number }[] = [];
	for (const file of files) {
		if (!chosen.has(file.key)) continue;
		if (reviews.needsReview(file.toon)) forReview.push({ key: file.key, version: file.latest });
		else live.push({ key: file.key, version: file.latest });
	}
	// the server's rules decide the rest (a catalog entry's toon, a physical change – Reviews): it names the keys that
	// need a review (submitted below) and the refusals, and publishes what it can
	if (live.length) {
		const result = await publishPartial(live);
		done.push(...result.published.map((p) => p.key));
		refused.push(...result.refused);
		forReview.push(...live.filter((w) => result.review.includes(w.key)));
	}
	let submitted = '';
	if (forReview.length) {
		const note = await askNote();
		if (note !== null) {
			const result = await contentApi.submitReview(forReview, note.trim());
			void reviews.load();
			submitted = m['creator.submitted']({ reviewers: result.reviewers.join(', ') });
		}
	}
	const notices = publishNotices(done, refused, label, {
		done: (count, names) => m['creator.publish_tree_done_named']({ count, names }),
		refused: (name, reason) => m['creator.publish_tree_refused']({ name, reason }),
		nothing: () => m['creator.nothing_to_publish']()
	}, submitted);
	for (const n of notices) toast(n.text, { type: n.type, kind: 'publish', ...(n.detail ? { detail: n.detail } : {}) });
	return { message: notices[0]?.text ?? '', published: done, refused, notices };
}

/** a publish that failed as a whole: its words, as the report's and the Log's line */
export function publishFailed(e: unknown): PublishTreeResult {
	const text = e instanceof Error ? e.message : String(e);
	toast(text, { type: 'error', kind: 'publish' });
	return { message: text, published: [], refused: [], notices: [{ type: 'error', text }] };
}
