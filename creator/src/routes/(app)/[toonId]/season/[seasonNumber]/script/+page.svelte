<script lang="ts">
	import { page } from '$app/state';
	import { entities } from '@scutoons/content';
	import ScriptRoute from '$lib/components/script/script-route.svelte';
	import { fileLabel } from '$lib/labels.ts';

	// a season's script (research/KANON.md §3, #81): how Cat Pope was written
	const toon = $derived(page.params.toonId as string);
	const entity = $derived(entities.seasonScript(toon, Number(page.params.seasonNumber)));
</script>

<ScriptRoute {toon} {entity} title={fileLabel(entity)} />
