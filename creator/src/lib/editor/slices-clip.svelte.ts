// Copy / cut / paste of slices (BACKLOG 10.39 step 5) as one handler for every place that picks slices: the chapter
// tree's slice list (region 'slices') and the phone's slice grid (the selection bar, owner 2026-10-12). The picked
// slices with their boxes, items, events and sounds; a paste goes after the last picked slice (else at the end) – fresh
// ids, media of another toon referenced (`../<toon>/…`); Shift+V puts the copied slice's audio + events onto every
// picked one.
import { getAt, type Path } from '@scutoons/content';
import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
import { applySliceSettings, clipPlacements, makeClip, mergeSounds, prepareSlices, remapSounds, soundsOf } from '$lib/editor/clip.ts';
import { clipWhat, pastedMessage, type ClipHandler } from '$lib/editor/clipboard.svelte.ts';
import { boxIdSource, getClipContext } from '$lib/editor/clip-context.ts';
import { m } from '$lib/paraglide/messages.js';

export interface SlicesClipOptions {
	doc: () => ContentDoc;
	groupPath: () => Path;
	slices: () => any[];
	/** the slices it acts on (a list's picks, else its shown slice) */
	picked: () => number[];
	/** the picks after a cut / paste */
	pick: (next: number[]) => void;
	/** the slice to show after a cut / paste */
	onselect: (index: number) => void;
}

export function slicesClipHandler(o: SlicesClipOptions): ClipHandler {
	const pickedNow = () => [...o.picked()].filter((i) => i < o.slices().length).sort((a, b) => a - b);
	const noSounds = () => ({ samples: new Map<string, string>(), tracks: new Map<string, string>() });
	return {
	copy() {
		const clipCtx = getClipContext();
		const list = pickedNow().map((i) => o.slices()[i]).filter(Boolean);
		if (!list.length || !clipCtx) return null;
		const entries = $state.snapshot(list);
		const clip = makeClip('slices', o.doc().toon, entries, { sounds: soundsOf(entries, clipCtx.chapter.get(clipCtx.chapter.primary, ['audio'])), chapter: clipCtx.chapterNumber });
		return { clip, what: clipWhat(clip) };
	},
	remove() {
		const gone = pickedNow();
		if (!gone.length) return;
		o.doc().edit((file) => {
			const list = getAt(file, [...o.groupPath(), 'slices']);
			if (Array.isArray(list)) for (const i of [...gone].reverse()) list.splice(i, 1);
		});
		o.pick([]);
		o.onselect(Math.max(0, Math.min(gone[0], o.slices().length - 1)));
	},
	paste(clip) {
		const clipCtx = getClipContext();
		if (clip.kind !== 'slices' || !clipCtx) return null;
		const refused = clipCtx.placementLimit(clipPlacements(clip));
		if (refused) return refused;
		const at = pickedNow().length ? Math.max(...pickedNow()) + 1 : o.slices().length;
		const placementIds = new Set<string>(o.slices().flatMap((s) => (s?.items ?? []).map((i: any) => i?.id)));
		const prepared = prepareSlices(clip, clip.entries, { toon: o.doc().toon, placementIds, nextBoxId: boxIdSource(clipCtx) });
		let map = noSounds();
		// the sounds go into the chapter's library (a separate scene: a second undo step in the chapter document)
		if (clip.sounds && o.doc() !== clipCtx.chapter) clipCtx.chapter.edit((file) => (map = mergeSounds(file, clip.sounds, clip.toon, o.doc().toon)));
		o.doc().edit((file) => {
			if (clip.sounds && o.doc() === clipCtx.chapter) map = mergeSounds(file, clip.sounds, clip.toon, o.doc().toon);
			for (const s of prepared) remapSounds(s, map);
			let list = getAt(file, [...o.groupPath(), 'slices']);
			if (!Array.isArray(list)) {
				const group = getAt(file, o.groupPath()) ?? file;
				group.slices = [];
				list = group.slices;
			}
			list.splice(at, 0, ...prepared);
		});
		o.pick(prepared.map((_, n) => at + n));
		o.onselect(at);
		return { message: pastedMessage(clip, o.doc().toon) };
	},
	pasteSettings(clip) {
		const clipCtx = getClipContext();
		if (clip.kind !== 'slices' || !clipCtx) return null;
		const targets = pickedNow();
		const source = clip.entries[0];
		if (!targets.length || !source) return { message: m['creator.clip_no_settings']() };
		let map = noSounds();
		if (clip.sounds && o.doc() !== clipCtx.chapter) clipCtx.chapter.edit((file) => (map = mergeSounds(file, clip.sounds, clip.toon, o.doc().toon)));
		o.doc().edit((file) => {
			if (clip.sounds && o.doc() === clipCtx.chapter) map = mergeSounds(file, clip.sounds, clip.toon, o.doc().toon);
			const list = getAt(file, [...o.groupPath(), 'slices']);
			if (!Array.isArray(list)) return;
			for (const i of targets) {
				if (!list[i]) continue;
				applySliceSettings(list[i], source);
				remapSounds(list[i], map);
			}
		});
		return { message: pastedMessage(clip, o.doc().toon, true, m['creator.clip_what_slices']({ count: targets.length })) };
	}
};
}
