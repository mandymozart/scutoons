import type { Reroute } from '@sveltejs/kit';
import { deLocalizeUrl } from '$lib/paraglide/runtime.js';

// `/de/…` and `/en/…` render the same routes (the locale comes from the URL prefix, paraglide.config.js)
export const reroute: Reroute = (request) => deLocalizeUrl(request.url).pathname;
