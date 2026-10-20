// Copy / paste on the slice canvas + its inspector (ONE region, 'slice' – BACKLOG 10.39 steps 2 + 3): the canvas
// selection (a text box, a placed item, a collage piece, a sound area, an event area) is what Ctrl+C / X take; Ctrl+V
// adds the copied entry to the open slice as a new one (fresh id, a step offset when it would sit exactly on another;
// a piece's file referenced across toons), Shift+V puts the copied entry's settings onto the selection of the same kind
// (a box's look, a placement's action …, a piece's look, a sound area's sound + trigger, an event area's event). VN
// events of a group script paste as slice events (when in view).
import { getAt, Trigger } from '@scutoons/content';
import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
import type { ChapterView } from '$lib/editor/chapter-state.svelte.ts';
import type { CanvasSelection } from '$lib/components/slice/slice-canvas.svelte';
import { nextDialogId } from '$lib/editor/defaults.ts';
import * as edits from '$lib/editor/slice-edit.ts';
import {
	applyBoxLook,
	applyItemSettings,
	applyPieceLook,
	applySampleSettings,
	applySliceEventSettings,
	dropEventIds,
	listAt,
	makeClip,
	mergeSounds,
	newPlacementId,
	offsetRect,
	preparePiece,
	soundsOf,
	type Clip
} from '$lib/editor/clip.ts';
import { clipWhat, pastedMessage, type ClipHandler, type ClipResult } from '$lib/editor/clipboard.svelte.ts';
import { m } from '$lib/paraglide/messages.js';

export interface SliceClipContext {
	view: ChapterView;
	/** the open slice (null: none open) */
	target: () => edits.SliceTarget | null;
	toon: string;
	chapterNumber: number;
	/** the chapter document: its `audio` holds the sound library a sound area names */
	chapter: ContentDoc;
	/** a paste that would place catalog items over their limit: the refusal (BACKLOG 10.39 step 7); null = fine */
	placementLimit?: (adds: Map<string, number>) => ClipResult | null;
}

/** each canvas kind's list in the slice (slice-edit.ts) */
const LISTS = edits.SLICE_LISTS;

export function sliceClipHandler(ctx: SliceClipContext): ClipHandler {
	const { view } = ctx;
	const entryOf = (t: edits.SliceTarget, sel: CanvasSelection) => getAt(t.slice, [...LISTS[sel.kind], sel.index]);

	return {
		copy() {
			const t = ctx.target();
			const sel = view.selection;
			if (!t || !sel) return null;
			const entry = entryOf(t, sel);
			if (!entry) return null;
			const sounds = sel.kind === 'sample' ? soundsOf([{ audio: { audio: { samples: [entry] } } }], ctx.chapter.get(ctx.chapter.primary, ['audio'])) : undefined;
			const clip = makeClip(sel.kind, ctx.toon, [entry], { sounds, chapter: ctx.chapterNumber });
			return { clip, what: clipWhat(clip) };
		},
		remove() {
			const t = ctx.target();
			if (!t || !view.selection) return;
			edits.removeSelected(t, view.selection);
			view.selection = null;
		},
		paste(clip) {
			const t = ctx.target();
			if (!t) return { message: m['creator.clip_no_target'](), error: false };
			const kind = clip.kind === 'events' ? 'event' : clip.kind;
			// a canvas kind only (slices / groups paste elsewhere)
			if (!Object.hasOwn(LISTS, kind)) return null;
			if (kind === 'item') {
				const adds = new Map<string, number>();
				for (const it of clip.entries) if (typeof it?.item === 'string') adds.set(it.item, (adds.get(it.item) ?? 0) + 1);
				const refused = ctx.placementLimit?.(adds);
				if (refused) return refused;
			}
			// a sound area's sound: into the chapter's library first (when the slice is in a separate scene, that is a
			// second undo step in the chapter document)
			let soundMap = { samples: new Map<string, string>(), tracks: new Map<string, string>() };
			if (kind === 'sample' && clip.sounds && t.doc !== ctx.chapter) ctx.chapter.edit((file) => (soundMap = mergeSounds(file, clip.sounds, clip.toon, ctx.toon)));
			const into = kind as CanvasSelection['kind'];
			let selected = -1;
			t.doc.edit((file) => {
				if (kind === 'sample' && clip.sounds && t.doc === ctx.chapter) soundMap = mergeSounds(file, clip.sounds, clip.toon, ctx.toon);
				const slice = getAt(file, t.slicePath);
				if (!slice) return;
				const list = listAt(slice, LISTS[into] as string[]);
				const placements = new Set<string>((slice.items ?? []).map((i: any) => i?.id));
				const pieceIds = new Set<string>((slice.pieces ?? []).map((p: any) => p?.id));
				for (const raw of clip.entries) {
					if (into === 'piece') {
						// a fresh id, an offset, its file referenced from this toon (clip.ts preparePiece)
						const piece = preparePiece(raw, clip.toon, ctx.toon, list, pieceIds);
						if (piece) list.push(piece);
						continue;
					}
					const entry = JSON.parse(JSON.stringify(raw));
					delete entry.layer; // lands on top of the layer stack
					if (kind === 'dialog') {
						const own = (slice.dialog ?? []).map((d: any) => d?.id).filter((id: unknown): id is string => typeof id === 'string');
						entry.id = nextDialogId(ctx.chapterNumber, [...(t.taken?.() ?? []), ...own]);
						entry.rect = offsetRect(entry.rect, list);
					} else if (kind === 'item') {
						entry.id = newPlacementId(String(entry.item ?? 'item'), placements);
						placements.add(entry.id);
						entry.rect = offsetRect(entry.rect, list);
						dropEventIds(entry.events);
					} else if (kind === 'sample') {
						if (soundMap.samples.has(entry.id)) entry.id = soundMap.samples.get(entry.id);
						if (entry.trigger?.rect) entry.trigger.rect = offsetRect(entry.trigger.rect, list);
					} else {
						delete entry.id; // a fresh one (assignEventIds in the edit)
						if (clip.kind === 'events' && !entry.trigger) entry.trigger = { type: Trigger.INTERSECT };
						else if (entry.trigger?.rect) entry.trigger.rect = offsetRect(entry.trigger.rect, list);
					}
					list.push(entry);
				}
				selected = list.length - 1;
			});
			if (selected >= 0) view.select({ kind: into, index: selected });
			return { message: pastedMessage(clip, ctx.toon) };
		},
		pasteSettings(clip) {
			const t = ctx.target();
			const sel = view.selection;
			if (!t || !sel) return { message: m['creator.clip_no_settings'](), error: false };
			const source = clip.entries[0];
			if (clip.kind !== sel.kind || !source) return { message: m['creator.clip_settings_kind']({ what: clipWhat(clip) }), error: false };
			let soundMap = { samples: new Map<string, string>(), tracks: new Map<string, string>() };
			if (sel.kind === 'sample' && clip.sounds && t.doc !== ctx.chapter) ctx.chapter.edit((file) => (soundMap = mergeSounds(file, clip.sounds, clip.toon, ctx.toon)));
			t.doc.edit((file) => {
				if (sel.kind === 'sample' && clip.sounds && t.doc === ctx.chapter) soundMap = mergeSounds(file, clip.sounds, clip.toon, ctx.toon);
				const target = getAt(file, [...t.slicePath, ...LISTS[sel.kind], sel.index]);
				if (!target) return;
				if (sel.kind === 'dialog') applyBoxLook(target, source);
				else if (sel.kind === 'item') applyItemSettings(target, source);
				else if (sel.kind === 'piece') applyPieceLook(target, source);
				else if (sel.kind === 'sample') {
					applySampleSettings(target, source);
					if (soundMap.samples.has(target.id)) target.id = soundMap.samples.get(target.id);
				} else applySliceEventSettings(target, source);
			});
			return { message: pastedMessage(clip, ctx.toon, true) };
		}
	};
}
