<script lang="ts">
	// The creator's root layout: only the tokens (incl. tools.css) and the page. The creator's frame (the session gate,
	// the shell, the menus, paraglide …) is the (app) group's layout – every page of the creator lives in routes/(app)/
	// (a group: the URLs stay the same). Outside it only the handheld camera's phone page (/h/<code>, #126; owner
	// 2026-10-20: "a barebone page so it doesnt need to load so much grap. it is only accessed through a qr code when
	// linked anyhow"), which loads none of the frame.
	import '@scutoons/tokens';
	import type { Snippet } from 'svelte';

	let { children }: { children: Snippet } = $props();
</script>

{@render children()}
