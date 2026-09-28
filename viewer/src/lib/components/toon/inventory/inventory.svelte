<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { localizeHref } from '$lib/paraglide/runtime.js';
	import { getNavigationContext } from '$lib/contexts/navigation.svelte';
	import { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import { getItemsUiContext } from '$lib/contexts/items-ui.svelte.ts';
	import type { Collectable as CollectableType } from '$lib/models/visual-novel.ts';
	import type { Entry } from '$lib/models/items.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';
	import { onClickOutside } from 'runed';
	import inventoryIcon from '$lib/icons/inventory.svg';
	import ItemIcon from '$lib/components/items/item-icon.svelte';
	import ItemDetail from '$lib/components/items/item-detail.svelte';
	import RecipePreview from '$lib/components/items/recipe-preview.svelte';
	import Avatar from '$lib/components/avatar/avatar.svelte';

	// toon collectables (legacy VN data) — used as icon fallback for items with the same id
	let { collectables = [] }: { collectables?: CollectableType[] } = $props();

	const nav = getNavigationContext();
	const inventory = getInventoryContext();
	const ui = getItemsUiContext();
	const inventoryVisible = $derived(nav.isOpen('inventory'));

	let inventoryElement = $state<HTMLDivElement>();
	let tab = $state<'items' | 'avatar'>('items');
	let selected = $state<Entry | null>(null);
	let combineWith = $state<Entry | null>(null);
	let combineSecond = $state<Entry | null>(null);

	const entries = $derived(
		[...inventory.entries].sort((a, b) => (a.acquired_at < b.acquired_at ? -1 : 1))
	);
	// keep the detail sheet in sync with inventory updates (e.g. qty change, entry consumed)
	$effect(() => {
		if (selected && !inventory.entries.some((e) => String(e.id) === String(selected!.id))) selected = null;
	});

	function iconsFor(entry: Entry): string[] {
		const legacy = (collectables as any[]).find((c) => c.id === entry.item_id)?.image;
		return [inventory.iconOf(entry), inventory.item(entry.item_id)?.icon ?? '', legacy ?? ''];
	}

	function reset() {
		selected = null;
		combineWith = null;
		combineSecond = null;
	}

	function pick(entry: Entry) {
		if (combineWith) {
			if (String(entry.id) === String(combineWith.id) && entry.qty < 2) return;
			combineSecond = entry;
			return;
		}
		selected = entry;
	}

	onClickOutside(
		() => inventoryElement,
		() => {
			// overlays opened from the inventory (camera, claim) live outside of it
			if (inventoryVisible && !ui.camera && !ui.claimCode && !ui.minigame) nav.closeOverlay();
		}
	);

	$effect(() => {
		function handleKeyDown(e: KeyboardEvent) {
			const target = e.target as HTMLElement | null;
			if (target?.closest('input, textarea, select, [contenteditable]')) return;
			if (ui.camera || ui.claimCode) return;
			if (e.key === 'Escape' && inventoryVisible) {
				nav.closeOverlay();
			} else if (e.key === 'i' || e.key === 'I') {
				nav.toggleOverlay('inventory');
			}
		}

		window.addEventListener('keydown', handleKeyDown);
		return () => window.removeEventListener('keydown', handleKeyDown);
	});

	// `?inventory=open` (QR landing page) or a claim that wants the inventory shown
	$effect(() => {
		if (ui.inventoryRequested) {
			nav.openOverlay('inventory');
			ui.requestInventory(false);
		}
	});

	onMount(() => {
		const param = page.url.searchParams.get('inventory');
		if (param === 'open' || param === '1' || param === 'true') {
			nav.openOverlay('inventory');
			const url = new URL(window.location.href);
			url.searchParams.delete('inventory');
			goto(url.pathname + url.search, { replaceState: true, noScroll: true, keepFocus: true });
		}
	});

	function toggleInventory(e?: MouseEvent) {
		e?.stopPropagation();
		nav.toggleOverlay('inventory');
	}
</script>

<button class="inventory-toggle" onclick={toggleInventory}>
	<img src={inventoryIcon} alt="Inventory" />
</button>

<div class="inventory {inventoryVisible ? 'visible' : 'hidden'}" bind:this={inventoryElement}>
	<img class="frame" src={asset(`catpope/images/vectors/vector-demon-saw-1.svg`)} alt="" />

	<div class="tabs" role="tablist">
		<button
			role="tab"
			aria-selected={tab === 'items'}
			class:active={tab === 'items'}
			onclick={() => ((tab = 'items'), reset())}>{m['items.tab_items']()}</button
		>
		<button
			role="tab"
			aria-selected={tab === 'avatar'}
			class:active={tab === 'avatar'}
			onclick={() => ((tab = 'avatar'), reset())}>{m['items.tab_avatar']()}</button
		>
	</div>

	<div class="sheet">
		{#if !inventory.isAccount}
			<p class="guest">
				{m['items.guest_hint']()}
				<a href={localizeHref('/account')}>{m['items.sign_in']()}</a>
			</p>
		{/if}

		{#if tab === 'avatar'}
			<Avatar />
		{:else if combineWith && combineSecond}
			<RecipePreview
				first={combineWith}
				second={combineSecond}
				oncancel={() => (combineSecond = null)}
				ondone={() => reset()}
			/>
		{:else if selected && !combineWith}
			<ItemDetail entry={selected} onback={() => (selected = null)} oncombine={(e) => (combineWith = e)} />
		{:else if !inventory.catalog && inventory.entries.length === 0}
			<p class="empty">{inventory.catalogError ? m['items.unavailable']() : m['items.loading']()}</p>
		{:else if entries.length === 0}
			<p class="empty">{m['inventory.empty']()}</p>
		{:else}
			{#if combineWith}
				<div class="combine-bar">
					<span>{m['items.combine_hint']({ name: inventory.nameOf(combineWith) })}</span>
					<button class="btn-secondary" onclick={() => (combineWith = null)}>{m['common.cancel']()}</button>
				</div>
			{/if}
			<section class="items">
				{#each entries as entry (entry.id)}
					<button
						class="item"
						class:source={combineWith && String(combineWith.id) === String(entry.id)}
						class:equipped={inventory.isEquipped(entry)}
						onclick={() => pick(entry)}
						title={inventory.nameOf(entry)}
					>
						<ItemIcon srcs={iconsFor(entry)} name={inventory.nameOf(entry)} />
						{#if entry.qty > 1}
							<div class="count">{entry.qty}</div>
						{/if}
						<span class="name">{inventory.nameOf(entry)}</span>
					</button>
				{/each}
			</section>
		{/if}
	</div>
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
		max-height: 85dvh;
		display: flex;
		flex-direction: column;
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

	.tabs {
		position: relative;
		z-index: 1;
		display: flex;
		gap: 0.5rem;
		padding: 1rem 1rem 0;
	}
	.tabs button {
		padding: 0.5rem 1rem;
		background: black;
		color: white;
		border: 2px solid white;
		border-bottom: none;
	}
	.tabs button.active {
		background: white;
		color: black;
	}

	.sheet {
		position: relative;
		z-index: 1;
		background: white;
		padding: 1rem;
		overflow-y: auto;
		overscroll-behavior: contain;
		min-height: 8rem;
	}

	.guest {
		margin: 0 0 1rem;
		font-size: 0.75rem;
		line-height: 1.3;
		color: #6b6b6b;
	}
	.guest a {
		color: black;
	}

	.empty {
		margin: 0;
	}

	.combine-bar {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-bottom: 1rem;
		font-size: 0.75rem;
		line-height: 1.3;
	}
	.combine-bar button {
		padding: 0.5rem 1rem;
		flex: none;
	}

	.items {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(5rem, 1fr));
		gap: 0.5rem;
	}
	.item {
		position: relative;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.25rem;
		padding: 0.25rem;
		background: white;
		color: black;
		border: 2px solid #ccc;
		cursor: pointer;
		touch-action: manipulation;
		transition: border-color 0.1s ease;
	}
	.item:hover {
		background: white;
		border-color: black;
	}
	.item.source {
		border-color: black;
		filter: drop-shadow(0 0 6px var(--primary-color));
	}
	.item.equipped::after {
		content: '';
		position: absolute;
		top: 0.25rem;
		right: 0.25rem;
		width: 0.5rem;
		height: 0.5rem;
		background: var(--primary-color);
		border: 1px solid black;
	}
	.name {
		font-size: 0.625rem;
		line-height: 1.2;
		text-align: center;
		overflow-wrap: anywhere;
	}
	.count {
		position: absolute;
		left: 0.25rem;
		top: 0.25rem;
		padding: 0 0.25rem;
		font-size: 0.75rem;
		background: white;
		border: 1px solid black;
		border-radius: 0.5rem;
	}
</style>
