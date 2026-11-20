// The editor kit's demo (shared/ui/src/editor): only on the dev server, never in a build
import { dev } from '$app/environment';
import { error } from '@sveltejs/kit';

export const load = () => {
	if (!dev) error(404, 'Not found');
};
