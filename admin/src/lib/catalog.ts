// The catalog (items, editions, toons, permissions) changes only with a deploy: loaded once per page load.
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
