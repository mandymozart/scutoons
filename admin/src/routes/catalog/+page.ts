// /catalog = the catalog's entry: its first kind (agents/CATALOG.md part 3). The old read-only page's `?tab=` links
// (items, editions, toons, permissions) land on that kind.
import { redirect } from '@sveltejs/kit';
import { EDITED_KINDS } from '$lib/catalog/kinds.ts';

export function load({ url }) {
	const tab = url.searchParams.get('tab') ?? '';
	const kind = (EDITED_KINDS as readonly string[]).includes(tab) ? tab : 'items';
	redirect(307, `/catalog/${kind}`);
}
