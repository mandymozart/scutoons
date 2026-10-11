/**
 * Publish down the tree (BACKLOG 10.60, owner 2026-10-11: "über jede höhere Hierarchie kann man das Zeug darunter
 * publishen – mit einer Liste, mit Checkboxen an- und abwählen; zusammenfassende Änderungen, aber trotzdem granular"):
 * every stored file below a level that has an unpublished draft, as a checklist (all ticked); the ticked ones are
 * published – or submitted for review where the account's review rules ask for it.
 *
 * Documents with parts (the map's places / routes / layers, the characters – DOCUMENT_PARTS) list one line per change
 * and can be published in part (owner 2026-10-12).
 *
 * Levels: everything (the platform: every toon + the world – map, characters, settings …) → a toon (all its files) → a
 * season (itself, its chapters and their scene files) → a chapter (itself and its scene files).
 */
import { checklistDialog, askDialog, type ChecklistGroup } from '@scutoons/ui';
import type { ContentIndexEntity } from '@scutoons/api';
import { contentApi, contentIndex, getPublished, getVersion, publishVersion, putDraft } from '$lib/api.ts';
import { DOCUMENT_PARTS, applyParts, diffParts, type PartChange } from '@scutoons/content';
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

/** a stored file with a draft newer than what readers see */
const hasDraft = (e: ContentIndexEntity) => e.published === null || e.latest > e.published;

/** is a key below the level (or the level's own file)? */
export function inScope(key: string, scope: PublishScope): boolean {
	const chapter = (t: string, n: number) => key === `chapter:${t}:${n}` || key.startsWith(`scene:${t}:${n}:`);
	switch (scope.kind) {
		case 'all':
			return true;
		case 'files':
			return scope.keys.includes(key);
		case 'toon':
			return key.split(':')[1] === scope.toon || key === `toon:${scope.toon}`;
		case 'season':
			return key === `season:${scope.toon}:${scope.number}` || scope.chapters.some((n) => chapter(scope.toon, n));
		case 'chapter':
			return chapter(scope.toon, scope.number);
	}
}

/** the order in a list: the level's own files first, then down the tree */
const KIND_ORDER = ['toon', 'entry', 'season', 'chapter', 'scene', 'script', 'cast', 'credits'];
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
	const lists = await Promise.all(toons.map((t) => contentIndex(t).then((r) => r.entities)));
	return lists.flat().filter(hasDraft).filter((e) => inScope(e.key, scope)).sort(byTree);
}

/** one line of the checklist: a whole file, or one change inside a document with parts (the map's places …) */
interface Line {
	id: string;
	file: ContentIndexEntity;
	change?: PartChange;
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
const PART_WORD: Record<string, () => string> = {
	locations: () => m['creator.publish_part_place'](),
	routes: () => m['creator.publish_part_route'](),
	layers: () => m['creator.publish_part_layer'](),
	characters: () => m['creator.publish_part_character']()
};
const KIND_WORD: Record<PartChange['kind'], () => string> = {
	added: () => m['creator.publish_part_added'](),
	changed: () => m['creator.publish_part_changed'](),
	removed: () => m['creator.publish_part_removed'](),
	settings: () => m['creator.publish_part_settings']()
};
const lineLabel = (l: Line) =>
	!l.change
		? label(l.file.key)
		: l.change.kind === 'settings'
			? `${label(l.file.key)} · ${KIND_WORD.settings()}`
			: `${label(l.file.key)} · ${PART_WORD[l.change.field ?? '']?.() ?? l.change.field}: ${nameOf(l.change.name, l.change.id)}`;
const lineDetail = (l: Line) => (l.change && l.change.kind !== 'settings' ? `${KIND_WORD[l.change.kind]()} · ${detail(l.file)}` : detail(l.file));

type Parts = Map<string, { published: unknown; draft: unknown; fields: readonly string[] }>;

/**
 * The lines for the files: a document with parts (DOCUMENT_PARTS – the map, the characters) that is published already
 * gives one line per change against its published version; every other file is one line.
 */
async function linesOf(files: ContentIndexEntity[]): Promise<{ lines: Line[]; parts: Parts }> {
	const lines: Line[] = [];
	const parts: Parts = new Map();
	for (const file of files) {
		const fields = DOCUMENT_PARTS[file.key];
		if (fields && file.published !== null) {
			const [pub, draft] = await Promise.all([getPublished(file.key), getVersion(file.key, file.latest)]);
			const changes = diffParts(pub.content, draft.content, fields);
			if (changes.length) {
				parts.set(file.key, { published: pub.content, draft: draft.content, fields });
				for (const change of changes) lines.push({ id: `${file.key}#${change.line}`, file, change });
				continue;
			}
		}
		lines.push({ id: file.key, file });
	}
	return { lines, parts };
}

/**
 * The checklist for a level, then publish / submit the ticked lines. Returns the message for the page (or null when
 * cancelled) and the keys that went live (whole or in part).
 */
export async function publishTree(scope: PublishScope, title: string): Promise<{ message: string; published: string[] } | null> {
	const files = await pendingFiles(scope);
	if (!files.length) return { message: m['creator.nothing_to_publish'](), published: [] };
	const { lines, parts } = await linesOf(files);
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
	const names = (ls: Line[]) => ls.map((l) => (l.change!.kind === 'settings' ? 'settings' : nameOf(l.change!.name, l.change!.id))).join(', ').slice(0, 180);
	for (const file of files) {
		const mine = lines.filter((l) => l.file === file);
		const ticked = mine.filter((l) => chosen.has(l.id));
		if (!ticked.length) continue;
		// the version the list showed – a newer save in between is not published unseen
		let version = file.latest;
		const p = parts.get(file.key);
		const inPart = !!p && ticked.length < mine.length;
		if (p && inPart) {
			// one document stays one document: save published + the ticked changes and put that live; the whole draft is
			// saved again on top, so what was not ticked stays a draft (owner 2026-10-12: "die Karte nicht mit ganz vielen Dokumenten")
			const merged = applyParts(p.published, p.draft, p.fields, new Set(ticked.map((l) => l.change!.line)));
			version = (await putDraft(file.key, merged, file.latest, `Published in part: ${names(ticked)}`)).version.id;
		}
		if (reviews.needsReview(file.toon)) forReview.push({ key: file.key, version });
		else {
			await publishVersion(file.key, version);
			done.push(file.key);
		}
		if (p && inPart) await putDraft(file.key, p.draft, version, `The draft goes on (not published yet: ${names(mine.filter((l) => !chosen.has(l.id)))})`);
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
