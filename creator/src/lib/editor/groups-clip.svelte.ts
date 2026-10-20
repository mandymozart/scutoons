// Copy / cut / paste of a chapter's groups (BACKLOG 10.39 step 5) as one handler for every place that picks groups: the
// chapter tree (region 'tree') and the phone's group list (the selection bar, owner 2026-10-12). The picked groups (else
// the open one); a paste goes after the last picked / the open group (else at the end), fresh ids, unique labels, media of another toon referenced; Shift+V
// puts the copied group's renderer, end and `continues` onto every picked group of the chapter.
import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
import { applyGroupSettings, clipPlacements, makeClip, mergeSounds, prepareGroup, remapSounds, soundsOf } from '$lib/editor/clip.ts';
import { clipWhat, pastedMessage, type ClipHandler } from '$lib/editor/clipboard.svelte.ts';
import { boxIdSource, getClipContext } from '$lib/editor/clip-context.ts';
import { m } from '$lib/paraglide/messages.js';

export interface GroupsClipOptions {
	chapter: () => ContentDoc;
	groups: () => any[];
	chapterNumber: () => number;
	/** the groups it acts on (the picks, else the open one) */
	picked: () => number[];
	/** the picks after a cut / paste */
	pick: (next: number[]) => void;
	/** after a cut: the removed indexes */
	onremove: (gone: number[]) => void;
	/** after a paste: where the first pasted group went */
	onpaste: (at: number) => void;
}

export function groupsClipHandler(o: GroupsClipOptions): ClipHandler {
	const pickedNow = () => [...o.picked()].filter((i) => i < o.groups().length).sort((a, b) => a - b);
	return {
	copy() {
		const clipCtx = getClipContext();
		const list = pickedNow().map((i) => o.groups()[i]).filter(Boolean);
		if (!list.length || !clipCtx) return null;
		const entries = $state.snapshot(list);
		const slices = entries.flatMap((g: any) => g?.slices ?? []);
		const clip = makeClip('groups', o.chapter().toon, entries, { sounds: soundsOf(slices, o.chapter().get(o.chapter().primary, ['audio'])), chapter: o.chapterNumber() });
		return { clip, what: clipWhat(clip) };
	},
	remove() {
		const gone = pickedNow();
		if (!gone.length) return;
		o.chapter().edit((file) => {
			if (Array.isArray(file.groups)) for (const i of [...gone].reverse()) file.groups.splice(i, 1);
		});
		o.pick([]);
		o.onremove(gone);
	},
	paste(clip) {
		const clipCtx = getClipContext();
		if (clip.kind !== 'groups' || !clipCtx) return null;
		const refused = clipCtx.placementLimit(clipPlacements(clip));
		if (refused) return refused;
		const now = pickedNow();
		const at = now.length ? Math.max(...now) + 1 : o.groups().length;
		const labels = new Set(clipCtx.groupLabels());
		const nextBoxId = boxIdSource(clipCtx);
		const prepared = clip.entries.map((g) => prepareGroup(clip, g, { toon: o.chapter().toon, nextBoxId }, labels));
		o.chapter().edit((file) => {
			const map = mergeSounds(file, clip.sounds, clip.toon, o.chapter().toon);
			for (const g of prepared) for (const s of g.slices ?? []) remapSounds(s, map);
			file.groups ??= [];
			file.groups.splice(at, 0, ...prepared);
		});
		o.onpaste(at);
		o.pick(prepared.map((_, n) => at + n));
		return { message: pastedMessage(clip, o.chapter().toon) };
	},
	pasteSettings(clip) {
		const clipCtx = getClipContext();
		if (clip.kind !== 'groups') return null;
		const source = clip.entries[0];
		const targets = pickedNow();
		if (!targets.length || !source) return { message: m['creator.clip_no_settings']() };
		o.chapter().edit((file) => {
			for (const i of targets) if (file.groups?.[i]) applyGroupSettings(file.groups[i], source);
		});
		return { message: pastedMessage(clip, o.chapter().toon, true, m['creator.clip_what_groups']({ count: targets.length })) };
	}
};
}
