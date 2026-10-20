<script lang="ts">
	import { getCreatorNav } from '$lib/nav.svelte.ts';
	import ToonList from '$lib/components/page/toon-list.svelte';
	import HomeBar from '$lib/components/page/home-bar.svelte';

	// the Toons area (CREATOR-NAV.md §2a): every toon; the menu lists them too, flat
	const nav = getCreatorNav();
</script>

<div class="creator-page">
	<main class="browser">
		<ToonList toons={nav.toons} columns />
	</main>
	<HomeBar toons={nav.toons} />
</div>

<style>
	.browser {
		padding: 1.5rem 1rem;
		max-width: 75rem;
	}
</style>
