// The custom collage presets (research/COLLAGE.md §12.4): ONE platform-wide, language-neutral document `colorations`
// ({ presets: CollagePreset[] } – the moods precedent, characters.ts). The chapter editor and the score stage load its
// newest draft: the canvas and the stage draw the pieces that name one with it (bakePresets – readers get them baked by
// the server), the fields copy-on-write from it, the checks judge against it; saved and published with the page.
import { emptyColorations, entities, NEUTRAL_LOCALE, type CollagePreset, type Colorations } from '@scutoons/content';
import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
import { ensure } from '$lib/editor/characters.ts';
import { neutralDoc } from '$lib/editor/languages.ts';

export const presetsDoc = () => neutralDoc(entities.colorations);

/** the stored presets in the grid's order, deleted ones included (none stored or not loaded yet: none) */
export function ownPresets(doc: ContentDoc | null | undefined): CollagePreset[] {
	const list = (doc?.data[NEUTRAL_LOCALE] as Colorations | null | undefined)?.presets;
	return Array.isArray(list) ? list : [];
}

/** the list a check judges custom ids against – undefined while it loads or when it failed to load (nothing judged) */
export const judgedPresets = (doc: ContentDoc): CollagePreset[] | undefined => (doc.loading || doc.error ? undefined : ownPresets(doc));

/** starts the document when the store has none yet (the first "Save as preset") */
export const ensurePresets = (doc: ContentDoc) => ensure(doc, emptyColorations);
