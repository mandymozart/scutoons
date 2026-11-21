<script lang="ts">
	import type { Snippet } from 'svelte';
	import { usePageDoc } from '$lib/shell/shell.svelte.ts';

	/** A page without documents of its own hands the bar's Publish menu its Publish while this is mounted */
	let { publish }: { publish: Snippet } = $props();
	usePageDoc(() => ({ publish }));
</script>
