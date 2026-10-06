// The chapter editor's slice edits (COMPONENTS.md step 12): add a text (with a preset's look), a box for an
// onomatopoeia event, a placed item, a ⚡ event, a sample area, remove the selection. Each one is one edit of the
// language-neutral document (texts start empty or in the source language) and answers what to select next.
import { deleteAt, getAt, previewText, Trigger, TriggerMode, type Path } from '@scutoons/content';
import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
import type { CanvasSelection } from '$lib/components/slice/slice-canvas.svelte';
import { newDialog, nextDialogId } from '$lib/editor/defaults.ts';
import { TEXT_PRESETS, applyTextPreset, type TextPreset } from '$lib/editor/text-presets.ts';

/** the slice being edited: its doc, its path, its number and data */
export interface SliceTarget {
	doc: ContentDoc;
	slicePath: Path;
	chapterNumber: number;
	sliceIndex: number;
	slice: any;
	/** every text box id of the chapter (its scenes too): a new id is none of them (owner 2026-10-06) */
	taken?: () => ReadonlySet<string>;
}

export const presetOf = (id: string) => TEXT_PRESETS.find((p) => p.id === id)!;

/** ids follow the image number like chapter 1 (s1c1-<slice>-<n>), else the position in the group */
function nextId(t: SliceTarget) {
	const imageNo = Number(String(t.slice?.image ?? '').match(/(\d+)\.\w+$/)?.[1]);
	return nextDialogId(t.chapterNumber, (imageNo || t.sliceIndex + 1) - 1, t.slice?.dialog ?? [], 1, t.taken?.());
}

/** a new text with a preset's look (default speech; a drawn one keeps the drawn rect) */
export function addDialog(t: SliceTarget, rect?: { x: number; y: number; width: number; height?: number }, preset: TextPreset = presetOf('speech')): CanvasSelection {
	const id = nextId(t);
	const index = (t.slice?.dialog ?? []).length;
	t.doc.edit((file) => {
		const slice = getAt(file, t.slicePath);
		if (!slice) return;
		slice.dialog ??= [];
		const dialog = newDialog(id, { ...(rect ?? preset.rect) });
		applyTextPreset(dialog as unknown as Record<string, unknown>, preset);
		slice.dialog.push(dialog);
	});
	return { kind: 'dialog', index };
}

/**
 * "Create a text box for this" (onomatopoeia event `index` of the slice's events, or of `eventsPath` = a placed item's
 * tap events, relative to the slice): a bare lettering box in the middle of the slice (white letters, ink outline,
 * uppercase, "BAM" in the source language – the other languages show it until they get their own), revealed by the event.
 */
export function createBoxForEvent(t: SliceTarget, index: number, eventsPath: Path = ['events']): CanvasSelection {
	const id = nextId(t);
	const boxIndex = (t.slice?.dialog ?? []).length;
	t.doc.edit((file) => {
		const slice = getAt(file, t.slicePath);
		const event = slice ? getAt(slice, eventsPath)?.[index] : undefined;
		if (!slice || event?.type !== 'showOnomatopeia') return;
		slice.dialog ??= [];
		const ono = presetOf('onomatopoeia');
		const dialog = { ...newDialog(id, { ...ono.rect }), text: { [t.doc.textSource]: 'BAM' } };
		applyTextPreset(dialog as unknown as Record<string, unknown>, ono);
		slice.dialog.push(dialog);
		(event.params ??= {}).dialog = id;
	});
	// the new box is selected: its text and look are set in the text box inspector
	return { kind: 'dialog', index: boxIndex };
}

/** places a catalog item at the spot (else in the middle of the slice; a quarter of its width) */
export function addSliceItem(t: SliceTarget, itemId: string, spot: { x: number; y: number } | null): CanvasSelection {
	const items: any[] = t.slice?.items ?? [];
	const taken = new Set(items.map((it) => it?.id));
	let n = 1;
	while (taken.has(`${itemId}-${n}`)) n++;
	t.doc.edit((file) => {
		const slice = getAt(file, t.slicePath);
		if (!slice) return;
		slice.items ??= [];
		slice.items.push({ id: `${itemId}-${n}`, item: itemId, rect: spot ? { x: Math.min(75, Math.max(0, spot.x)), y: Math.min(95, Math.max(0, spot.y)), width: 25 } : { x: 37.5, y: 40, width: 25 } });
	});
	return { kind: 'item', index: items.length };
}

/**
 * the ⚡ tool (owner 2026-10-03): a drawn area → a click event there; a plain click (no area) → a whole-slice event
 * that fires when the slice scrolls into view. A message to start with – its type, trigger and area change in Events.
 */
export function addSliceEvent(t: SliceTarget, rect: { x: number; y: number; width: number; height: number }): CanvasSelection {
	const area = rect.width > 0 && rect.height > 0;
	const index = t.slice?.events?.length ?? 0;
	t.doc.edit((file) => {
		const slice = getAt(file, t.slicePath);
		if (!slice) return;
		slice.events ??= [];
		slice.events.push({ type: 'showMessage', params: { text: {}, wait: true }, trigger: area ? { type: Trigger.CLICK, rect: { ...rect } } : { type: Trigger.INTERSECT } });
	});
	return { kind: 'event', index };
}

/** a click sample on a drawn area (the chapter's first library sample); null without a sample library */
export function addSampleArea(t: SliceTarget, rect: { x: number; y: number; width: number; height: number }, library: { id: string }[]): CanvasSelection | null {
	if (!library.length) return null;
	t.doc.edit((file) => {
		const slice = getAt(file, t.slicePath);
		if (!slice) return;
		slice.audio ??= {};
		slice.audio.audio ??= {};
		slice.audio.audio.samples ??= [];
		slice.audio.audio.samples.push({ id: library[0].id, trigger: { type: Trigger.CLICK, mode: TriggerMode.POLY, rect: { ...rect } }, volume: 0.6 });
	});
	return { kind: 'sample', index: (t.slice?.audio?.audio?.samples?.length ?? 0) };
}

/** removes the selected text / item / sample area / event (an emptied list goes) */
export function removeSelected(t: SliceTarget, selection: CanvasSelection) {
	const { kind, index } = selection;
	const listPath =
		kind === 'dialog'
			? [...t.slicePath, 'dialog']
			: kind === 'item'
				? [...t.slicePath, 'items']
				: kind === 'sample'
					? [...t.slicePath, 'audio', 'audio', 'samples']
					: [...t.slicePath, 'events'];
	t.doc.edit((file) => {
		const list = getAt(file, listPath);
		if (!Array.isArray(list)) return;
		list.splice(index, 1);
		if (!list.length) deleteAt(file, listPath, t.slicePath.length);
	});
}

/** text boxes as choices for an onomatopoeia event (`params.dialog`): id + the start of the text (`chain`: the languages to try) */
export const boxChoices = (slices: any[] | undefined, chain: readonly string[] = []) =>
	(slices ?? []).flatMap((sl: any) =>
		(sl?.dialog ?? [])
			.filter((d: any) => typeof d?.id === 'string' && d.id)
			.map((d: any) => ({ id: d.id as string, label: `${d.id} · ${previewText(d.text, chain).slice(0, 24)}` }))
	);
