<script lang="ts">
	import PickerSheet from '$lib/components/controls/picker-sheet.svelte';
	import { gridArrows } from '$lib/editor/grid-keys.ts';
	import type { ItemCategory, ItemDef } from '@scutoons/content';
	import { itemCatalog, itemName, loadCatalog } from '$lib/editor/catalog.svelte.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The item selector as a drawer like the asset manager and the character selector (owner 2026-10-03): search,
	 * category, cards with icon + name; a click picks. `exclude`: ids already used (related items) – shown as taken.
	 * The drawer can be dragged wider (shared Sheet, up to half the window).
	 */
	let {
		open = $bindable(false),
		title,
		exclude = [],
		onpick
	}: {
		open?: boolean;
		title?: string;
		exclude?: string[];
		onpick: (item: ItemDef) => void;
	} = $props();

	let query = $state('');
	let category = $state<ItemCategory | ''>('');

	$effect(() => {
		if (!open) return;
		query = '';
		void loadCatalog();
	});

	const items = $derived(itemCatalog.catalog?.items ?? []);
	const categories = $derived([...new Set(items.map((i) => i.category))].sort());
	const shown = $derived.by(() => {
		const q = query.trim().toLowerCase();
		return items
			.filter((i) => !category || i.category === category)
			.filter((i) => !q || i.id.includes(q) || itemName(i, i.id).toLowerCase().includes(q))
			.sort((a, b) => itemName(a, a.id).localeCompare(itemName(b, b.id)));
	});

	function pick(item: ItemDef) {
		onpick(item);
		open = false;
	}
</script>

<PickerSheet bind:open title={title ?? m['creator.items_picker_title']()} testid="item-picker-sheet">
		<div class="sc-picker-filters">
			<!-- svelte-ignore a11y_autofocus -->
			<input type="search" bind:value={query} placeholder={m['creator.items_search']()} autofocus data-testid="item-search" />
			<select value={category} onchange={(e) => (category = e.currentTarget.value as ItemCategory | '')} aria-label={m['creator.items_category']()}>
				<option value="">{m['creator.items_all_categories']()}</option>
				{#each categories as c (c)}<option value={c}>{c}</option>{/each}
			</select>
		</div>
		{#if !itemCatalog.loaded}
			<p class="hint">{m['creator.related_items_loading']()}</p>
		{:else}
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div class="sc-cards" onkeydown={gridArrows}>
				{#each shown as item (item.id)}
					{@const taken = exclude.includes(item.id)}
					<button type="button" class="sc-card" data-control-exempt="picture card" data-cell disabled={taken} onclick={() => pick(item)} title={item.id} data-testid="item-card-{item.id}">
						<span class="sc-card-img">{#if item.icon}<img src={asset(item.icon)} alt="" loading="lazy" />{/if}</span>
						<span class="name">{itemName(item, item.id)}</span>
						<span class="hint">{taken ? m['creator.items_taken']() : item.category}</span>
					</button>
				{:else}
					<p class="hint">{m['creator.items_none']()}</p>
				{/each}
			</div>
		{/if}
</PickerSheet>

<style>
	.name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		max-width: 100%;
	}
	.hint {
		color: var(--muted);
		font-size: var(--type-caption-size);
	}
</style>
