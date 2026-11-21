import { error } from '@sveltejs/kit';
import { ApiError, fetchExperience } from '$lib/api.ts';
import { placeIn } from '$lib/xr/xr-session.ts';
import type { PageLoad } from './$types.js';

/** `/xr/<experience>` and `/xr/<experience>/<scene or target>`: the published experience and where the URL lands in it */
export const load: PageLoad = async ({ params }) => {
	const experience = await fetchExperience(params.experience).catch((e: unknown) => {
		if (e instanceof ApiError && e.status === 404) error(404, 'Not found');
		throw error(e instanceof ApiError && e.status ? e.status : 503, `Failed to load experience ${params.experience}`);
	});
	const place = placeIn(experience.configuration, params.at);
	if (!place) error(404, 'Not found');
	return { experience, place };
};
