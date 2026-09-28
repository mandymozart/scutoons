<script lang="ts">
	import type { MinigameProps } from './registry.ts';
	import { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	let { entry, onclose }: MinigameProps = $props();
	const inventory = getInventoryContext();
</script>

<div class="placeholder">
	{#if entry}<h2>{inventory.nameOf(entry)}</h2>{/if}
	<p>{m['items.minigame_soon']()}</p>
	<button onclick={onclose}>{m['items.close']()}</button>
</div>

<style>
	.placeholder {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		align-items: flex-start;
	}
	h2 {
		margin: 0;
		font-size: 1rem;
		font-weight: 700;
	}
	p {
		margin: 0;
	}
</style>
