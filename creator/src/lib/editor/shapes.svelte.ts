// The vector library for the collage picker (agents/research/COLLAGE.md §5, VECTORS §6 step 8): every shape of the
// asset manager in one request, once per session (436 locally – small rows, no paging in the picker); a failed load is
// tried again the next time the picker opens.
import type { AssetSummary } from '@scutoons/api';
import { assetsApi } from '$lib/api.ts';

/** the folder the library lives in (the shapes are catpope's, shared by every toon) */
export const SHAPE_FOLDER = 'catpope';

let items = $state<AssetSummary[] | null>(null);
let error = $state('');
let pending: Promise<void> | null = null;

export const shapeLibrary = {
	/** null = not loaded yet */
	get items() {
		return items;
	},
	get error() {
		return error;
	},
	get loading() {
		return !items && !error;
	},
	load(): Promise<void> {
		if (items) return Promise.resolve();
		if (pending) return pending;
		error = '';
		pending = assetsApi
			.list({ kind: 'shape', folder: SHAPE_FOLDER, page_size: 500 })
			.then((list) => {
				items = list.items;
				error = '';
			})
			.catch((e: unknown) => {
				error = e instanceof Error ? e.message : String(e);
			})
			.finally(() => {
				pending = null;
			});
		return pending;
	}
};
