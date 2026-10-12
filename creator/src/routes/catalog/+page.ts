import { redirect } from '@sveltejs/kit';

// the catalog opens on its items (agents/CATALOG-IN-CREATOR.md)
export function load() {
	redirect(307, '/catalog/items');
}
