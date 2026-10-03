<script lang="ts">
	import { Sheet } from '@scutoons/ui';
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

<Sheet bind:open title={title ?? m['creator.items_picker_title']()} side="right" closeLabel={m['creator.characters_close']()} resizeLabel={m['creator.resize_panel']()}>
	<div class="picker" data-testid="item-picker-sheet">
		<div class="filters">
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
			<div class="cards">
				{#each shown as item (item.id)}
					{@const taken = exclude.includes(item.id)}
					<button type="button" class="card" data-control-exempt="picture card" disabled={taken} onclick={() => pick(item)} title={item.id} data-testid="item-card-{item.id}">
						<span class="img">{#if item.icon}<img src={asset(item.icon)} alt="" loading="lazy" />{/if}</span>
						<span class="name">{itemName(item, item.id)}</span>
						<span class="hint">{taken ? m['creator.items_taken']() : item.category}</span>
					</button>
				{:else}
					<p class="hint">{m['creator.items_none']()}</p>
				{/each}
			</div>
		{/if}
	</div>
</Sheet>

<style>
	.picker {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		min-width: 0;
	}
	.filters {
		display: grid;
		grid-template-columns: minmax(0, 2fr) minmax(0, 1fr);
		gap: 0.5rem;
	}
	.cards {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(8rem, 1fr));
		gap: 0.4rem;
	}
	.card {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 0.25rem;
		padding: 0.4rem;
		text-align: left;
		background: var(--paper);
		color: var(--ink);
		border: var(--border-width-hairline) solid var(--ink);
		cursor: pointer;
	}
	.card:hover:not(:disabled),
	.card:focus-visible {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: -2px;
	}
	.card:disabled {
		opacity: 0.45;
		cursor: default;
	}
	.img {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 100%;
		aspect-ratio: 1;
	}
	.img img {
		max-width: 80%;
		max-height: 80%;
	}
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
