<script lang="ts">
	// The creator's root layout: only the tokens (incl. tools.css) and the page. The creator's frame (the session gate,
	// the shell, the menus, paraglide …) is the (app) group's layout – every page of the creator lives in routes/(app)/
	// (a group: the URLs stay the same). Outside it only the handheld camera's phone page (/h/<code>, #126; owner
	// 2026-10-20: "a barebone page so it doesnt need to load so much grap. it is only accessed through a qr code when
	// linked anyhow"), which loads none of the frame.
	import '@scutoons/tokens';
	import type { Snippet } from 'svelte';
	import { MediaQuery } from 'svelte/reactivity';
	import { beforeNavigate } from '$app/navigation';
	import { page, updated } from '$app/state';
	import DesktopNotice from '$lib/components/shell/desktop-notice.svelte';
	import CoreUnitsProvider from '$lib/components/fields/core-units-provider.svelte';
	import CoreHistoryProvider from '$lib/components/page/core-history-provider.svelte';
	import CorePlacesProvider from '$lib/components/page/core-places-provider.svelte';

	let { children }: { children: Snippet } = $props();

	// The creator is desktop-only (RULES H9): below 744 CSS px only the notice. A page opened that narrow never mounts; a
	// window narrowed later keeps the page mounted but hidden, so nothing unsaved is lost. The handheld page is a phone's.
	const narrow = new MediaQuery('width < 744px');
	const handheld = $derived(page.route.id?.startsWith('/h/') ?? false);
	const away = $derived(narrow.current && !handheld);
	let started = $state(false);
	$effect.pre(() => {
		if (!away) started = true;
	});

	// A tab open across a deploy (#493, owner 2026-11-12: a 500 on /catpope and the chapter editor): its next in-app
	// navigation would import a page file the deploy removed and end on "500 Internal Error". When a newer build is out
	// (SvelteKit's version poll), the navigation becomes a full page load of the new build instead; a failed preload
	// (the same missing file) reloads the page.
	beforeNavigate(({ willUnload, to }) => {
		if (updated.current && !willUnload && to?.url) location.href = to.url.href;
	});
	if (typeof window !== 'undefined') window.addEventListener('vite:preloadError', () => location.reload());
</script>

{#if started || !away}
	<div class="page" class:away><CoreUnitsProvider>{@render children()}</CoreUnitsProvider></div>
{/if}
<CoreHistoryProvider />
<CorePlacesProvider />
{#if away}<DesktopNotice />{/if}

<style>
	.page {
		display: contents;
	}
	.page.away {
		display: none;
	}
</style>
