// /catalog = the admin's part of the catalog ("Shop & codes"): its first kind. The old read-only page's `?tab=` links
// land on that kind when the admin still has it; items, editions … are the creator's now (the page links there).
import { redirect } from '@sveltejs/kit';
import { ADMIN_KINDS } from '$lib/catalog/admin-kinds.ts';

export function load({ url }) {
	const tab = url.searchParams.get('tab') ?? '';
	const kind = (ADMIN_KINDS as readonly string[]).includes(tab) ? tab : ADMIN_KINDS[0];
	redirect(307, `/catalog/${kind}`);
}
