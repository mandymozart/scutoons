<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import type { Group } from '$lib/models/content.ts';
	import { getGameContext } from '$lib/contexts/game.svelte';
	import { getVisualNovelContext } from '$lib/contexts/visual-novel.svelte';
	import Characters from '../vn/characters.svelte';
	import Onomatopeias from '../vn/onomatopeias.svelte';
	import MessageBox from '../vn/message.svelte';
	import { EventProcessor } from '$lib/visual-novel/event-processor.ts';

	let { group }: { group: Group } = $props();

	const game = getGameContext();
	const vn = getVisualNovelContext();
	
	let cancelled = false;

	onMount(async () => {
		cancelled = false;
		const processor = new EventProcessor(group.events, game, vn);
		const jumpTo = await processor.run();
		
		if (!cancelled && jumpTo) {
			console.log(`✨ Navigating to: ${jumpTo}`);
			goto(jumpTo);
		}
	});
	
	onDestroy(() => {
		cancelled = true;
		vn.clear();
		console.log('🧹 VN component unmounted, state cleared');
	});
</script>

<Characters />
<Onomatopeias />
<MessageBox />

<style>
</style>
