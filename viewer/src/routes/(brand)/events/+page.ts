import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types.ts';
import { fetchEvents } from '$lib/api.ts';

/** Events from the API's library (agents/DATA.md). */
export const load: PageLoad = async () => {
	try {
		return { events: await fetchEvents() };
	} catch {
		error(404, 'Not found');
	}
};
