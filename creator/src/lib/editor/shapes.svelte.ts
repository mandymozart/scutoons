// The vector library for the collage picker (agents/research/COLLAGE.md §5, VECTORS §6 step 8): every shape of the
// asset manager, once per session – 500 rows a request (the server's most), the next pages until the total (436 today;
// no paging in the picker); a failed load is tried again the next time the picker opens. Loaded by the picker when it
// opens and by a list that holds a shape (its title is the piece's name – collage-words.ts pieceLabel).
import type { AssetSummary } from '@scutoons/api';
import { assetsApi } from '$lib/api.ts';
import { allPages } from '$lib/editor/collage-pick.ts';

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
		pending = allPages((page) => assetsApi.list({ kind: 'shape', folder: SHAPE_FOLDER, page, page_size: 500 }))
			.then((all) => {
				items = all;
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
