<script lang="ts">
	import { page } from '$app/state';
	import ChapterEditor from '$lib/components/chapter-editor.svelte';

	const toon = $derived(page.params.toonId as string);
	const chapterNumber = $derived(Number(page.params.chapterNumber));
</script>

{#key `${toon}/${chapterNumber}`}
	<ChapterEditor {toon} {chapterNumber} />
{/key}
