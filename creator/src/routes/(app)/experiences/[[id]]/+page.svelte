<script lang="ts">
	import { page } from '$app/state';
	import ExperienceBrowser from '$lib/components/experiences/experience-browser.svelte';

	const id = $derived(page.params.id ?? null);
</script>

<ExperienceBrowser {id} />
