<script lang="ts">
	import SequenceNavigation from '$lib/components/toon/sequence-navigation.svelte';
	import { goto } from '$app/navigation';
	import { setContext, onDestroy } from 'svelte';
	import { setAudioContext } from '$lib/contexts/audio.svelte.ts';
	import { setGameContext } from '$lib/contexts/game.svelte';
	import { setVisualNovelContext } from '$lib/contexts/visual-novel.svelte';
	import { setNavigationContext } from '$lib/contexts/navigation.svelte';
	import Map from '$lib/components/toon/map/map.svelte';
	import Inventory from '$lib/components/toon/inventory/inventory.svelte';

	let { children, data, params } = $props();
	let sequence = data.sequence || [];

	function navigate(url: string | URL, opts?: Parameters<typeof goto>[1]): void {
		goto(url, opts);
	}
	setContext('navigation', navigate);

	const audio = setAudioContext(params.toonId);
	const game = setGameContext(params.toonId);
	const vn = setVisualNovelContext();
	const nav = setNavigationContext();
	
	vn.setCharacters(data.characters || []);

	onDestroy(async () => {
		console.log('🎵 Toon layout destroyed, cleaning up audio');
		await audio.unload();
	});
	$inspect(vn.state.sceneCharacters,game.state.variables);
</script>

{@render children()}

<SequenceNavigation {sequence} {params} /> 
<Map />
<Inventory />
