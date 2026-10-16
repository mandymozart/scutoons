// What an item shows in 3D (ITEMS.md "3D item models"): a `.glb`, or the shared record model with the item's look
// (`VinylLook` – @scutoons/media/vinyl-model builds it). The 3D views (item-model.svelte, record-stage.js) load either.
import type { ModelSource } from '@scutoons/content';
import type { ModelBuilder } from '@scutoons/media/model-view';
import { buildVinyl, vinylPictures } from '@scutoons/media/vinyl-model';
import { asset } from '$lib/utils/cdn.ts';

/** what the 3D views load: a .glb's URL, or the builder of the shared record model */
export function modelLoader(source: ModelSource): string | ModelBuilder {
	return typeof source === 'string' ? asset(source) : (THREE) => buildVinyl(THREE, source, asset);
}

/** the files a model downloads (URLs): the .glb, or the look's pictures */
export function modelFiles(source: ModelSource | null | undefined): string[] {
	if (!source) return [];
	return (typeof source === 'string' ? [source] : vinylPictures(source)).map(asset);
}
