<script lang="ts">
	import { getGameContext } from '$lib/contexts/game.svelte';
	import { getNavigationContext } from '$lib/contexts/navigation.svelte';
	import type { Collectable as CollectableType } from '$lib/models/visual-novel.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';
	import Collectable from './collectable.svelte';
	import { onClickOutside } from 'runed';

	let { collectables }: { collectables: CollectableType[] } = $props();

	const game = getGameContext();
	const nav = getNavigationContext();
	const inventoryVisible = $derived(nav.isOpen('inventory'));

	let inventoryElement = $state<HTMLDivElement>();

	onClickOutside(
		() => inventoryElement,
		() => {
			if (inventoryVisible) nav.closeOverlay();
		}
	);

	$effect(() => {
		function handleKeyDown(e: KeyboardEvent) {
			if (e.key === 'Escape' && inventoryVisible) {
				nav.closeOverlay();
			} else if (e.key === 'i' || e.key === 'I') {
				nav.toggleOverlay('inventory');
			}
		}

		window.addEventListener('keydown', handleKeyDown);
		return () => window.removeEventListener('keydown', handleKeyDown);
	});

	function toggleInventory(e?: MouseEvent) {
		e?.stopPropagation();
		nav.toggleOverlay('inventory');
	}
	$inspect(collectables);
</script>

<button class="inventory-toggle" onclick={toggleInventory}>
	<img src={'/icons/inventory.svg'} alt="Inventory" />
</button>

<div class="inventory {inventoryVisible ? 'visible' : 'hidden'}" bind:this={inventoryElement}>
	<img class="frame" src={asset(`catpope/images/vectors/vector-demon-saw-1.svg`)} alt="" />
	{#if Object.keys(game?.state.variables).length > 0}
		<section class="items">
			{#each Object.entries(game.state.variables) as [key, value]}
				{#each collectables as collectable}
					{#if collectable.id === key}
						<div class="item">
							<Collectable {collectable} />
							<div class="count">{value}</div>
						</div>
					{/if}
				{/each}
			{/each}
		</section>
	{:else}
		<p class="empty">{m['inventory.empty']()}</p>
	{/if}
</div>

<style>
	.inventory-toggle {
		position: fixed;
		top: 0.5rem;
		right: 0.5rem;
		z-index: 1001;
		background: white;
		border: 2px solid black;
		padding: 0;
		border-radius: 50%;
		height: 2rem;
		width: 2rem;
		display: flex;
		cursor: pointer;
		transition: all 0.2s ease;
		align-items: center;
		justify-content: center;
		touch-action: manipulation;
	}
	.inventory-toggle img {
		width: 1rem;
	}
	.inventory-toggle:hover {
		background: var(--primary-color);
	}

	.inventory {
		position: fixed;
		left: 0;
		right: 0;
		bottom: 0;
		z-index: 1000;
		background: black;
		border-top: 2px solid black;
		/* padding: 1rem; */
		transform: translate(0, 140%);
		transition: transform 0.3s
			linear(0, 0.996 33.2%, 0.875 39.4%, 0.84 44.4%, 0.999 57.8%, 0.946 65%, 1 74.6%, 1);
		.frame {
			position: absolute;
			pointer-events: none;
			top: -15px;
			right: -10px;
			height: 100vw;
			transform: rotate(90deg);
			transform-origin: top;
			transition: transform 1s
			linear(0, 0.996 33.2%, 0.875 39.4%, 0.84 44.4%, 0.999 57.8%, 0.946 65%, 1 74.6%, 1);
			filter: invert(1);
			/* or to blue */
			filter: drop-shadow(0px 0px 13px var(--primary-color));
		}
	}
	.inventory.visible {
		transform: translate(0, 0);
		.frame {
			transform: rotate(90deg) translateY(-5px);
			transform-origin: top;
		}
	}

	.empty {
		margin: 0;
		padding: 1rem;
		color: white;
	}

	.items {
		background: black;
		padding: 1rem;
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(5rem, 1fr));
		gap: 1rem;
		z-index: 1;
		position: relative;
	}
	.item {
		/* display: flex;
		align-items: center;
		gap: 1rem; */
		position: relative;
	}
	.count {
		position: absolute;
		left: 0;
		top: 0;
		padding: 0 0.25rem;
		font-size: 0.75rem;
		background: white;
		border: 1px solid black;
		border-radius: 0.5rem;
	}
</style>
