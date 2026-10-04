// The catalog (items, editions, toons, permissions) lives in the database and is edited on the Catalog pages
// (agents/CATALOG.md part 3): loaded once and kept until a catalog write calls invalidateCatalog() – the next
// loadCatalog() (e.g. the codes / batches pages' edition filters) then fetches it again.
import type { AdminCatalog } from '@scutoons/api';
import { admin } from '$lib/api.ts';

let cached: Promise<AdminCatalog> | null = null;

export function loadCatalog(): Promise<AdminCatalog> {
	cached ??= admin.catalog().catch((error) => {
		cached = null;
		throw error;
	});
	return cached;
}

/** after every catalog write (put / delete / reorder / undo): the next loadCatalog() reads the server again */
export function invalidateCatalog(): void {
	cached = null;
}

/** Code types of the legacy generate form (+ what the permissions map knows) */
export const CODE_TYPES = ['membership', 'event', 'xr', 'promo', 'page'];

/** Filter values of GET /admin/codes `status` */
export const CODE_STATUSES = ['claimed', 'unclaimed', 'revoked', 'expired', 'active', 'signed', 'unsigned'] as const;

export interface Edition {
	id: string;
	toon: string;
	name: string;
	description?: string;
	edition_number?: number;
	edition_count: number;
	excluded_number?: number;
	creators?: string;
	[key: string]: unknown;
}
