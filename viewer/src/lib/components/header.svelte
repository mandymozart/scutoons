<script lang="ts">
	import type { ToonIndex } from '$lib/models/content.ts';
	import MainMenu from './main-menu.svelte';

	let { toons }: { toons: ToonIndex[] } = $props();
</script>

<MainMenu {toons} />
