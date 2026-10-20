// What the chapter editor's copy regions share (BACKLOG 10.39): the chapter document (its `audio` holds the sound
// library), the toon, the chapter's text box ids (a pasted box gets a fresh one) and its group labels, the item-limit
// check. The chapter editor sets it while it lives; the slice list and the chapter tree read it (the tree is rendered
// in the shell's sidebar through a snippet – a page-wide holder instead of a Svelte context).
import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
import type { ClipResult } from '$lib/editor/clipboard.svelte.ts';
import { nextDialogId } from '$lib/editor/defaults.ts';
import { shared } from '$lib/utils/shared.ts';

export interface ClipContext {
	chapter: ContentDoc;
	toon: string;
	chapterNumber: number;
	/** every text box id of the chapter and its scenes */
	boxIds: () => ReadonlySet<string>;
	/** every group label of the chapter (scenes resolved) */
	groupLabels: () => string[];
	/** a paste that would place catalog items over their limit: the refusal; null = fine */
	placementLimit: (adds: Map<string, number>) => ClipResult | null;
}

const holder = shared('clip-context', () => ({ current: null as ClipContext | null }));

/** the chapter editor's: set while it is mounted (call in an $effect – the cleanup clears it) */
export function setClipContext(ctx: ClipContext): () => void {
	holder.current = ctx;
	return () => {
		if (holder.current === ctx) holder.current = null;
	};
}
/** the open chapter's context (null: no chapter editor on the page) */
export const getClipContext = (): ClipContext | null => holder.current;

/**
 * A text box id source for one paste: the chapter counter like `nextId` in slice-edit.ts (s1c2-t<n>), unique in the
 * chapter and among the ids this paste already handed out.
 */
export function boxIdSource(ctx: Pick<ClipContext, 'boxIds' | 'chapterNumber'>) {
	const used = new Set(ctx.boxIds());
	return () => {
		const id = nextDialogId(ctx.chapterNumber, used);
		used.add(id);
		return id;
	};
}
