<script lang="ts">
	import { page } from '$app/state';
	import { entities } from '@scutoons/content';
	import ScriptRoute from '$lib/components/script/script-route.svelte';
	import { fileLabel } from '$lib/labels.ts';

	// a chapter's script (research/KANON.md §3, #81): how Top Toy is written
	const toon = $derived(page.params.toonId as string);
	const entity = $derived(entities.chapterScript(toon, Number(page.params.chapterNumber)));
</script>

<ScriptRoute {toon} {entity} title={fileLabel(entity)} />
