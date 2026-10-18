// The version note's suggestion for a page's unsaved files (DESIGN.md "Version notes", owner 2026-10-18): version-note.ts
// diffs each dirty file against its stored version, this module gives it the UI language's words (paraglide) and the
// names (the item catalog, the collage pieces). Deterministic – nothing is sent anywhere.
import { parseEntityKey } from '@scutoons/content';
import { m } from '$lib/paraglide/messages.js';
import { docLabel } from '$lib/labels.ts';
import { itemCatalog, itemName } from '$lib/editor/catalog.svelte.ts';
import { pieceLabel } from '$lib/editor/collage-words.ts';
import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
import {
	NOTE_MAX,
	chapterChanges,
	groupChanges,
	metaChanges,
	summarizeChanges,
	type GroupRef,
	type NoteChange,
	type NoteContext,
	type NoteKind,
	type NoteWords
} from '$lib/editor/version-note.ts';

type Words = Record<string, ((params?: Record<string, unknown>) => string) | undefined>;
const word = (key: string, params?: Record<string, unknown>) => (m as unknown as Words)[`creator.${key}`]?.(params);

const groupName = (g: GroupRef) =>
	'scene' in g ? m['creator.note_scene']({ name: g.scene }) : 'label' in g ? m['creator.note_group_label']({ name: g.label }) : m['creator.note_group_number']({ number: g.number });

export const noteWords: NoteWords = {
	subject: (kind: NoteKind, count: number) => (count === 1 ? (word(`note_${kind}`) ?? kind) : (word(`note_${kind}_many`, { count }) ?? `${count} ${kind}`)),
	verb: (action, kind) => (kind === 'item' && action === 'added' ? m['creator.note_placed']() : (word(`note_${action}`) ?? action)),
	phrase: (subject, verb, place) => m['creator.note_phrase']({ subject, verb, place }),
	onSlice: (number) => m['creator.note_on_slice']({ number }),
	inGroup: (group) => m['creator.note_in']({ group: groupName(group) }),
	group: groupName,
	slice: (number) => m['creator.note_slice']({ number }),
	newSlices: (range, count) => (count === 1 ? m['creator.note_new_slice']({ range }) : m['creator.note_new_slices']({ range })),
	removedSlices: (range, count) => (count === 1 ? m['creator.note_removed_slice']({ range }) : m['creator.note_removed_slices']({ range })),
	translated: (languages) => m['creator.note_translated']({ languages: languages.join(', ') }),
	plural: (kind) => (word(`note_${kind}_many`, { count: '' }) ?? kind).trim(),
	field: (key) => word(`note_field_${key}`) ?? null,
	get settings() {
		return m['creator.note_settings']();
	},
	get created() {
		return m['creator.note_created']();
	}
};

const parse = (json: string | undefined) => {
	if (!json) return null;
	try {
		return JSON.parse(json);
	} catch {
		return null;
	}
};

/** one file's changes since its stored version (every dirty locale; a per-language file's other languages = translated) */
function docChanges(doc: ContentDoc): NoteChange[] {
	const ctx: NoteContext = {
		languages: doc.texts,
		itemName: (id) => itemName(itemCatalog.catalog?.items.find((i) => i.id === id), id),
		pieceName: (piece) => pieceLabel(piece, null, null)
	};
	const out: NoteChange[] = [];
	for (const l of doc.dirtyLocales) {
		const before = parse(doc.saved[l]);
		const after = parse(JSON.stringify(doc.data[l]));
		if (!doc.neutral && l !== doc.primary && doc.locales.length > 1) {
			out.push({ kind: 'text', action: 'translated', lang: l });
			continue;
		}
		if (doc.kind === 'chapter') out.push(...chapterChanges(before, after, ctx));
		else if (doc.kind === 'group') {
			const scene = parseEntityKey(doc.entity)?.scene;
			out.push(...(before ? groupChanges(before, after, scene ? { scene } : undefined, ctx) : [{ kind: 'created' as const, action: 'added' as const, place: scene ? { group: { scene } } : undefined }]));
		} else out.push(...metaChanges(before, after, ctx));
	}
	return out;
}

/**
 * The suggested note for saving `docs` (their dirty files): the chapter and its scenes as one line, a meta file's own
 * line – several files' lines joined with "; ", each named when there is more than one. '' when nothing is dirty.
 */
export function suggestNote(docs: ContentDoc[]): string {
	const dirty = docs.filter((d) => d.dirty);
	const story = dirty.filter((d) => d.kind === 'chapter' || d.kind === 'group');
	const meta = dirty.filter((d) => !story.includes(d));
	const chapter = docs.find((d) => d.kind === 'chapter');
	const groups = chapter ? (chapter.data[chapter.primary]?.groups?.length ?? 0) : docs.filter((d) => d.kind === 'group').length;
	const parts: { label: string; text: string }[] = [];
	if (story.length) {
		const text = summarizeChanges(story.flatMap(docChanges), noteWords, { multipleGroups: groups > 1 });
		if (text) parts.push({ label: chapter ? docLabel(chapter) : docLabel(story[0]), text });
	}
	for (const doc of meta) {
		const text = summarizeChanges(docChanges(doc), noteWords);
		if (text) parts.push({ label: docLabel(doc), text });
	}
	const line = parts.length > 1 ? parts.map((p) => `${p.label}: ${p.text}`).join('; ') : (parts[0]?.text ?? '');
	return line.length > NOTE_MAX ? `${line.slice(0, NOTE_MAX - 1).trimEnd()}…` : line;
}
