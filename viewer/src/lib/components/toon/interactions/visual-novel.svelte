<script lang="ts">
	import { onMount } from 'svelte';
	import type { Group } from '$lib/models/content.ts';
	import { getGameContext } from '$lib/contexts/game.svelte';
	import { getVisualNovelContext } from '$lib/contexts/visual-novel.svelte';
	import { EventProcessor } from '$lib/visual-novel/event-processor.ts';
	import MessageBox from '../vn/message.svelte';
	import Characters from '../vn/characters.svelte';
	import Onomatopeias from '../vn/onomatopeias.svelte';

	let { group, onComplete }: { group: Group; onComplete: (jumpTo: string | null) => void } =
		$props();

	const game = getGameContext();
	const vn = getVisualNovelContext();

	onMount(async () => {
		const processor = new EventProcessor(group.events, game, vn);
		const jumpTo = await processor.run();
		
		onComplete(jumpTo);
	});
</script>

<Characters />
<Onomatopeias />
<MessageBox />

<style>

</style>
