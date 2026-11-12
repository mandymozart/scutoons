<script lang="ts">
	import { page } from '$app/state';
	import SceneBrowser from '$lib/components/scenes/scene-browser.svelte';

	// the deck scenes (#490, draft 3's first slice): the list beside the menu, the open scene `/scenes/<id>`
	const id = $derived(page.params.id ?? null);
</script>

<SceneBrowser {id} />
