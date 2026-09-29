<script lang="ts">
	import type { ToonIndex } from '@scutoons/content';
	import MainMenu from './main-menu.svelte';

	let { toons }: { toons: ToonIndex[] } = $props();
</script>

<MainMenu {toons} />
