/**
 * Publish down the tree (BACKLOG 10.60, owner 2026-10-11: "über jede höhere Hierarchie kann man das Zeug darunter
 * publishen – mit einer Liste, mit Checkboxen an- und abwählen; zusammenfassende Änderungen, aber trotzdem granular"):
 * every stored file below a level that has an unpublished draft, as a checklist (all ticked); the ticked ones are
 * published – or submitted for review where the account's review rules ask for it.
 *
 * Levels: everything (the platform: every toon + the world – map, characters, settings …) → a toon (all its files) → a
 * season (itself, its chapters and their scene files) → a chapter (itself and its scene files).
 */
import { checklistDialog, askDialog, type ChecklistGroup } from '@scutoons/ui';
import type { ContentIndexEntity } from '@scutoons/api';
import { contentApi, contentIndex, publishVersion } from '$lib/api.ts';
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

/**
 * an old per-language copy of a language-neutral document (before 2026-10-06: `chapter:t:n:de`, `scene:t:n:x:en`,
 * `season:t:n:de`, `toon-settings:t:de`) – read only now, never published again
 */
export function isLegacy(key: string): boolean {
	const parts = key.split(':');
	if (parts[0] === 'chapter' || parts[0] === 'season') return parts.length === 4;
	if (parts[0] === 'scene') return parts.length === 5;
	return parts[0] === 'toon-settings';
}

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
	return lists.flat().filter(hasDraft).filter((e) => !isLegacy(e.key) && inScope(e.key, scope)).sort(byTree);
}

/**
 * The checklist for a level, then publish / submit the ticked files. Returns the message for the page (or null when
 * cancelled) and the keys that went live.
 */
export async function publishTree(scope: PublishScope, title: string): Promise<{ message: string; published: string[] } | null> {
	const files = await pendingFiles(scope);
	if (!files.length) return { message: m['creator.nothing_to_publish'](), published: [] };
	const toonName = (id: string | null) => (id ? (scope.kind === 'all' ? scope.toons.find((t) => t.id === id)?.name : undefined) ?? id : m['creator.publish_tree_world']());
	const groups: ChecklistGroup[] =
		scope.kind === 'all'
			? [...new Set(files.map((f) => f.toon ?? ''))].map((t) => ({ label: toonName(t || null), items: files.filter((f) => (f.toon ?? '') === t).map((f) => ({ id: f.key, label: label(f.key), detail: detail(f) })) }))
			: [{ items: files.map((f) => ({ id: f.key, label: label(f.key), detail: detail(f) })) }];
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
	const chosen = files.filter((f) => picked.includes(f.key));
	const toReview = chosen.filter((f) => reviews.needsReview(f.toon));
	const toPublish = chosen.filter((f) => !reviews.needsReview(f.toon));
	const done: string[] = [];
	for (const f of toPublish) {
		// the version the list showed – a newer save in between is not published unseen
		await publishVersion(f.key, f.latest);
		done.push(f.key);
	}
	let submitted = '';
	if (toReview.length) {
		const note = await askDialog({ title: m['creator.submit_title'](), label: m['creator.submit_note_label'](), placeholder: m['creator.submit_note_placeholder'](), confirmLabel: m['creator.submit_send'](), skipLabel: m['creator.submit_send']() });
		if (note !== null) {
			const result = await contentApi.submitReview(toReview.map((f) => ({ key: f.key, version: f.latest })), note.trim());
			void reviews.load();
			submitted = m['creator.submitted']({ reviewers: result.reviewers.join(', ') });
		}
	}
	const message = [done.length ? m['creator.publish_tree_done']({ count: done.length }) : '', submitted].filter(Boolean).join(' ');
	return { message: message || m['creator.nothing_to_publish'](), published: done };
}
