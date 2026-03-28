import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types.ts';
import type { Event } from '$lib/models/content.ts';

export const load: PageLoad = async ({ fetch }) => {
	const response = await fetch(`/api/events.json`);
	const events: Event[] = await response.json();
	if (events) return { events };
	error(404, 'Not found');
};
