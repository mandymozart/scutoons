// isolation.svelte.ts (#298): call `isolateThisPage()` once in the script of a page that needs cross-origin isolation
// (isolation.ts). Arriving without the headers (a client-side move from another creator page) loads the page afresh once;
// moving on to a page that must not be isolated is a full load. Without the headers from the host (production not set up
// yet) the page stays as it is – the engine falls back to its slower transport and says so.
import { onMount } from 'svelte';
import { beforeNavigate } from '$app/navigation';
import { mustLeave, shouldReload } from './isolation.ts';

const KEY = 'scutoons.isolation.reload';

export function isolateThisPage(): void {
	onMount(() => {
		const isolated = crossOriginIsolated === true;
		let tried = false;
		try {
			tried = sessionStorage.getItem(KEY) === location.pathname;
			if (isolated) sessionStorage.removeItem(KEY);
			else if (shouldReload(isolated, tried)) sessionStorage.setItem(KEY, location.pathname);
		} catch {
			/* no storage: one reload at most per page view – `tried` stays false, so guard by the flag below */
			tried = true;
		}
		if (shouldReload(isolated, tried)) location.reload();
	});
	beforeNavigate(({ to, willUnload, cancel }) => {
		if (willUnload || !to) return;
		if (mustLeave(crossOriginIsolated === true, to.url.pathname)) {
			cancel();
			location.assign(to.url.href);
		}
	});
}
