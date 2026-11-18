import { error } from '@sveltejs/kit';
import { allCases, findCase, nextCase, written } from '$lib/docs/catalog.ts';
import { GROUPS } from '$lib/docs/tutorials.ts';
import type { EntryGenerator, PageLoad } from './$types.js';

export const entries: EntryGenerator = () =>
	allCases(GROUPS)
		.filter(written)
		.map((c) => ({ slug: c.slug }));

export const load: PageLoad = ({ params }) => {
	const found = findCase(GROUPS, params.slug);
	if (!found || !written(found.item)) error(404, 'No such tutorial');
	return { ...found, next: nextCase(GROUPS, params.slug) };
};
