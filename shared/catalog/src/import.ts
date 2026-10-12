// Bulk "Import images …" of the items page (agents/CATALOG.md part 3, the creator importers' pattern): files named
// `<item-id>.<ext>` or `<item-id>_<variant>.<ext>` (case-insensitive) → the media standard (`standardImage`:
// WebP, scaled down to at most 1024 px on the longest side, transparency kept; a WebP within that keeps its bytes) → upload, or a new version of the file already there, at
// `<toon or scutoons>/items/<item-id>[_<variant>].webp` → the item's (or variant's) `icon`, one PUT per item.
import type { CatalogEntry } from '@scutoons/api';
import { standardImage, uploadOrReplace } from '@scutoons/media';
import type { Data } from './kinds.ts';

export const MAX_SIDE = 1024;

export type ImageMatch = { item: CatalogEntry; variant: string | null; path: string } | { problem: 'unknown_item' | 'unknown_variant'; item?: string };

/** which item / variant a file name means, and where its WebP goes */
export function matchItemImage(name: string, items: CatalogEntry[]): ImageMatch {
	const stem = name.replace(/\.[^.]+$/, '').trim().toLowerCase();
	const path = (item: CatalogEntry, variant: string | null) => `${item.data?.toon || 'scutoons'}/items/${item.id}${variant ? `_${variant}` : ''}.webp`;
	const exact = items.find((i) => i.id.toLowerCase() === stem);
	if (exact) return { item: exact, variant: null, path: path(exact, null) };
	// the longest item id first: `dago_hoodie_black` = item dago_hoodie, variant black (not an item `dago`)
	const candidates = items.filter((i) => stem.startsWith(`${i.id.toLowerCase()}_`)).sort((a, b) => b.id.length - a.id.length);
	for (const item of candidates) {
		const rest = stem.slice(item.id.length + 1);
		const variant = (Array.isArray(item.data?.variants) ? item.data.variants : []).find((v: Data) => String(v?.id ?? '').toLowerCase() === rest);
		if (variant) return { item, variant: String(variant.id), path: path(item, String(variant.id)) };
	}
	return candidates.length ? { problem: 'unknown_variant', item: candidates[0].id } : { problem: 'unknown_item' };
}

/**
 * How an import stores a changed entry: the creator saves a draft of the entry's document (published through the one
 * checklist – agents/research/CATALOG-IN-CREATOR.md); moved here from the admin with the catalog
 */
export type CatalogWrite = (kind: 'items' | 'recipes', id: string, data: Data, note: string) => Promise<unknown>;

export interface ImportReport {
	uploaded: number;
	replaced: number;
	unchanged: number;
	/** the items whose icon(s) were set */
	updated: string[];
	/** per file: what went wrong (the caller words the unknown names) */
	problems: { file: string; problem: 'unknown_item' | 'unknown_variant' | 'unknown_recipe' | 'failed'; item?: string; message?: string }[];
}

export async function importItemImages(files: File[], items: CatalogEntry[], write: CatalogWrite, progress?: (done: number, total: number) => void): Promise<ImportReport> {
	const report: ImportReport = { uploaded: 0, replaced: 0, unchanged: 0, updated: [], problems: [] };
	/** item id → { the entry, the icons to set: variant id (null = the item's own) → path } */
	const changes = new Map<string, { entry: CatalogEntry; icons: Map<string | null, string> }>();
	let done = 0;
	for (const file of files) {
		const match = matchItemImage(file.name, items);
		try {
			if ('problem' in match) {
				report.problems.push({ file: file.name, problem: match.problem, item: match.item });
				continue;
			}
			const { blob } = await standardImage(file, { maxSide: MAX_SIDE });
			const result = await uploadOrReplace(blob, match.path, 'catalog image import');
			report[result]++;
			const change = changes.get(match.item.id) ?? { entry: match.item, icons: new Map() };
			change.icons.set(match.variant, match.path);
			changes.set(match.item.id, change);
		} catch (e) {
			report.problems.push({ file: file.name, problem: 'failed', message: e instanceof Error ? e.message : String(e) });
		} finally {
			progress?.(++done, files.length);
		}
	}
	// one PUT per item, all its variants in it
	for (const [id, { entry, icons }] of changes) {
		const data: Data = JSON.parse(JSON.stringify(entry.data));
		let changed = false;
		for (const [variant, path] of icons) {
			if (variant === null) {
				changed ||= data.icon !== path;
				data.icon = path;
			} else {
				const v = (data.variants as Data[]).find((x) => x.id === variant);
				if (v) {
					changed ||= v.icon !== path;
					v.icon = path;
				}
			}
		}
		if (!changed) continue;
		try {
			await write('items', id, data, 'image import');
			report.updated.push(id);
		} catch (e) {
			report.problems.push({ file: id, problem: 'failed', message: e instanceof Error ? e.message : String(e) });
		}
	}
	return report;
}

/** where a recipe's own picture goes: the output item's toon folder (`scutoons` for a shared item), `items/recipes/` */
export const recipeImagePath = (recipe: CatalogEntry, items: CatalogEntry[]) =>
	`${items.find((i) => i.id === recipe.data?.output?.item)?.data?.toon || 'scutoons'}/items/recipes/${recipe.id}.webp`;

/**
 * "Import images …" of the recipes page (owner 2026-10-04: "recipes also have images"): files named `<recipe-id>.<ext>`
 * (case-insensitive) → WebP (as the item import) at `recipeImagePath` → the recipe's `image`, one PUT per recipe.
 */
export async function importRecipeImages(files: File[], recipes: CatalogEntry[], items: CatalogEntry[], write: CatalogWrite, progress?: (done: number, total: number) => void): Promise<ImportReport> {
	const report: ImportReport = { uploaded: 0, replaced: 0, unchanged: 0, updated: [], problems: [] };
	let done = 0;
	for (const file of files) {
		const stem = file.name.replace(/\.[^.]+$/, '').trim().toLowerCase();
		const recipe = recipes.find((r) => r.id.toLowerCase() === stem);
		try {
			if (!recipe) {
				report.problems.push({ file: file.name, problem: 'unknown_recipe' });
				continue;
			}
			const { blob } = await standardImage(file, { maxSide: MAX_SIDE });
			const path = recipeImagePath(recipe, items);
			report[await uploadOrReplace(blob, path, 'catalog image import')]++;
			if (recipe.data?.image !== path) {
				await write('recipes', recipe.id, { ...JSON.parse(JSON.stringify(recipe.data)), image: path }, 'image import');
				report.updated.push(recipe.id);
			}
		} catch (e) {
			report.problems.push({ file: file.name, problem: 'failed', message: e instanceof Error ? e.message : String(e) });
		} finally {
			progress?.(++done, files.length);
		}
	}
	return report;
}
