<script lang="ts">
	// The creator's root layout: only the tokens (incl. tools.css) and the page. The creator's frame (the session gate,
	// the shell, the menus, paraglide …) is the (app) group's layout – every page of the creator lives in routes/(app)/
	// (a group: the URLs stay the same). Outside it only the handheld camera's phone page (/h/<code>, #126; owner
	// 2026-10-20: "a barebone page so it doesnt need to load so much grap. it is only accessed through a qr code when
	// linked anyhow"), which loads none of the frame.
	import '@scutoons/tokens';
	import type { Snippet } from 'svelte';
	import { beforeNavigate } from '$app/navigation';
	import { updated } from '$app/state';

	let { children }: { children: Snippet } = $props();

	// A tab open across a deploy (#493, owner 2026-11-12: a 500 on /catpope and the chapter editor): its next in-app
	// navigation would import a page file the deploy removed and end on "500 Internal Error". When a newer build is out
	// (SvelteKit's version poll), the navigation becomes a full page load of the new build instead; a failed preload
	// (the same missing file) reloads the page.
	beforeNavigate(({ willUnload, to }) => {
		if (updated.current && !willUnload && to?.url) location.href = to.url.href;
	});
	if (typeof window !== 'undefined') window.addEventListener('vite:preloadError', () => location.reload());
</script>

{@render children()}
