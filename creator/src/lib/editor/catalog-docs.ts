// The catalog in the creator (agents/CATALOG-IN-CREATOR.md, owner 2026-10-12: "eigentlich müsste der logischerweise
// komplett in den Creator umgezogen werden"): items, recipes, albums, products and editions, each entry its own
// versioned document (`item:<id>` …, server CatalogDocs) – drafts, history, the one publish checklist and the review
// rules like every other file. The forms are @scutoons/catalog's (shared with the admin during the move).
import { CATALOG_DOC_KINDS, type CatalogDoc, type CatalogDocKind, type CatalogEntry } from '@scutoons/api';
import type { CatalogLists } from '@scutoons/catalog';
import { contentApi } from '$lib/api.ts';
import { m } from '$lib/paraglide/messages.js';

export const CATALOG_PAGE_KINDS = Object.keys(CATALOG_DOC_KINDS) as CatalogDocKind[];

export const CATALOG_LABELS: Record<CatalogDocKind, () => string> = {
	items: () => m['creator.catalog_items'](),
	recipes: () => m['creator.catalog_recipes'](),
	albums: () => m['creator.catalog_albums'](),
	products: () => m['creator.catalog_products'](),
	editions: () => m['creator.catalog_editions']()
};

export const isCatalogPageKind = (kind: string | undefined): kind is CatalogDocKind => !!kind && (CATALOG_PAGE_KINDS as string[]).includes(kind);

/** an entry's document key: items + camera → `item:camera` */
export const catalogKey = (kind: CatalogDocKind, id: string) => `${CATALOG_DOC_KINDS[kind]}:${id}`;

/** every kind's entries as documents (the newest version of each) */
export async function loadCatalogDocs(): Promise<Record<CatalogDocKind, CatalogDoc[]>> {
	const lists = await Promise.all(CATALOG_PAGE_KINDS.map((k) => contentApi.catalogDocs(k).then((r) => r.entries)));
	return Object.fromEntries(CATALOG_PAGE_KINDS.map((k, i) => [k, lists[i]])) as Record<CatalogDocKind, CatalogDoc[]>;
}

/**
 * The lists the forms read (selects of items, editions, products, albums …) and the catalog problems run over – the
 * drafts as they stand; `open` replaces one entry by the form's data (problems follow the typing)
 */
export function catalogLists(
	docs: Partial<Record<CatalogDocKind, CatalogDoc[]>>,
	toons: { id: string; name: string }[],
	open?: { kind: CatalogDocKind; id: string; data: unknown }
): CatalogLists {
	// the toon selects of the forms (an entry's `toon`): the creator's toons
	const out: CatalogLists = { toons: toons.map((t, i) => ({ id: t.id, position: i, data: { id: t.id, name: t.name }, updated_at: '', updated_by: null })) };
	for (const kind of CATALOG_PAGE_KINDS) {
		const list = docs[kind];
		if (!list) continue;
		const entries: CatalogEntry[] = list.map((e, i) => ({
			id: e.id,
			position: i,
			data: (open && open.kind === kind && open.id === e.id ? open.data : e.content) as Record<string, any>,
			updated_at: e.latest_at ?? '',
			updated_by: e.latest_author
		}));
		if (open && open.kind === kind && !list.some((e) => e.id === open.id)) {
			entries.push({ id: open.id, position: entries.length, data: open.data as Record<string, any>, updated_at: '', updated_by: null });
		}
		out[kind] = entries;
	}
	return out;
}
