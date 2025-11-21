<script lang="ts">
	import { getGameContext } from '$lib/contexts/game.svelte';
	import { getNavigationContext } from '$lib/contexts/navigation.svelte';
	import { asset } from '$lib/utils/cdn.ts';
	import Collectable from './collectable.svelte';
	import { onClickOutside } from 'runed';

	const COLLECTABLES = [
		{
			id: 'cucumbers',
			name: 'Cucumbers',
			description: 'Cucumbers are food, weapon, toy, and decoration.',
			image: '/catpope/images/collectables/cucumber.png'
		}
	];

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
</script>

<button class="inventory-toggle" onclick={toggleInventory}>
	<img src={'/icons/inventory.svg'} alt="Inventory" />
</button>

<div class="inventory {inventoryVisible ? 'visible' : 'hidden'}" bind:this={inventoryElement}>
	<section class="items">
		{#if Object.keys(game.state.variables).length > 0}
			{#each Object.entries(game.state.variables) as [key, value]}
				{#each COLLECTABLES as collectable}
					{#if collectable.id === key}
						<div class="item">
							<Collectable {collectable} />
							<div class="count">{value}</div>
						</div>
					{/if}
				{/each}
			{/each}
		{:else}
			<p class="empty">Beessss wax, nothing here!</p>
		{/if}
	</section>
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
		background: white;
		border-top: 2px solid black;
		padding: 1rem;
		transform: translate(0, 100%);
		transition: transform 0.1s ease;
	}
	.inventory.visible {
		transform: translate(0, 0);
	}

	.items {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(5rem, 1fr));
		gap: 1rem;
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
		font-size: .75rem;
		background: white;
		border: 1px solid black;
		border-radius: .5rem;
	}
</style>
