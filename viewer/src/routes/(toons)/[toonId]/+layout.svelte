<script lang="ts">
	import SequenceNavigation from '$lib/components/toon/sequence-navigation.svelte';
	import { onDestroy } from 'svelte';
	import { setAudioContext } from '$lib/contexts/audio.svelte.ts';
	import { setVisualNovelContext } from '$lib/contexts/visual-novel.svelte';
	import Map from '$lib/components/toon/map/map.svelte';
	import Inventory from '$lib/components/toon/inventory/inventory.svelte';
	import ReaderEffect from '$lib/components/items/reader-effect.svelte';
	import { setGameContext } from '$lib/contexts/game.svelte.ts';
	import { setNavigationContext } from '$lib/contexts/navigation.svelte.ts';

	let { children, data, params } = $props();
	let sequence = data.sequence || [];

	const audio = setAudioContext(params.toonId);
	const vn = setVisualNovelContext();
	const game = setGameContext(params.toonId);
	const nav = setNavigationContext();

	vn.setCharacters(data.characters || []);

	onDestroy(async () => {
		await audio.unload();
	});
</script>

{@render children()}

<SequenceNavigation {sequence} {params} />
<Map />
<Inventory />
<ReaderEffect />
