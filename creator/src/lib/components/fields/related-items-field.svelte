<script lang="ts">
	import { movedAt } from '$lib/editor/move.ts';
	import ReorderButtons from '$lib/components/controls/reorder-buttons.svelte';
	import { onMount } from 'svelte';
	import { CloseIcon, Icon } from '@scutoons/ui';
	import type { Catalog, ItemDef } from '@scutoons/content';
	import { loadCatalog } from '$lib/editor/catalog.svelte.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import ItemPickerSheet from '$lib/components/items/item-picker-sheet.svelte';

	/**
	 * `related_items` of a season or a chapter (agents/ITEMS.md "Players, albums, related items"): catalog item ids in
	 * order, language-neutral – one list written to every locale (like the season's download / trailer and the chapter's
	 * featuring). Chips with the item's icon and name, reorder, remove; "Add an item …" opens the item selector.
	 */
	let {
		doc,
		hint,
		testid = 'related-items',
		legend = true
	}: { doc: ContentDoc; hint: string; testid?: string; /** false under a section heading that already names it */ legend?: boolean } = $props();

	let catalog = $state<Catalog | null>(null);
	let choosing = $state(false);
	let loading = $state(true);
	onMount(() => {
		loadCatalog().then((c) => {
			catalog = c;
			loading = false;
		});
	});

	const locale = getLocale();
	const list = $derived<string[]>(Array.isArray(doc.data[doc.primary]?.related_items) ? doc.data[doc.primary].related_items : []);
	const byId = $derived(new Map((catalog?.items ?? []).map((i) => [i.id, i])));
	const nameOf = (item: ItemDef | undefined, id: string) => (item ? (item.name[locale] ?? item.name.en ?? item.name.de ?? id) : id);

	function set(next: string[]) {
		doc.structural((file) => {
			if (next.length) file.related_items = [...next];
			else delete file.related_items;
		}, 'related_items');
	}
	function add(id: string) {
		if (!id || list.includes(id)) return;
		set([...list, id]);
	}
	const remove = (id: string) => set(list.filter((x) => x !== id));
	function move(from: number, to: number) {
		const next = movedAt(list, from, to);
		if (next) set(next);
	}
</script>

<fieldset class="related" data-testid={testid}>
	{#if legend}<legend>{m['creator.related_items']()}</legend>{/if}
	<p class="hint">{hint}</p>
	<ul class="chips">
		{#each list as id, i (id)}
			{@const item = byId.get(id)}
			{@const name = nameOf(item, id)}
			<li class="sc-chip-control has-image" class:is-warning={!loading && !item} data-testid="{testid}-{id}" title={id}>
				<span class="icon">{#if item?.icon}<img src={asset(item.icon)} alt="" />{/if}</span>
				<span class="name">{name}</span>
				{#if !loading && !item}<span class="hint">{m['creator.related_items_unknown']()}</span>{/if}
				<ReorderButtons index={i} count={list.length} onmove={move} axis="horizontal" />
				<button type="button" class="ghost" onclick={() => remove(id)} aria-label={m['creator.related_items_remove']({ name })} title={m['creator.related_items_remove']({ name })}><CloseIcon /></button>
			</li>
		{:else}
			<li class="hint">{m['creator.related_items_empty']()}</li>
		{/each}
	</ul>
	<!-- the item selector as a drawer like the asset manager / character selector (owner 2026-10-03) -->
	<button type="button" class="secondary add" disabled={loading || !catalog} onclick={() => (choosing = true)} data-testid="{testid}-add">
		{loading ? m['creator.related_items_loading']() : m['creator.related_items_add']()}
	</button>
	<ItemPickerSheet bind:open={choosing} title={m['creator.related_items_add']()} exclude={list} onpick={(item) => add(item.id)} />
</fieldset>

<style>
	.add {
		align-self: flex-start;
	}
	.related {
		border: 0;
		padding: 0;
		margin: 0;
		/* a fieldset is as wide as its content by default: let narrow columns wrap the chips and the hint */
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.related legend {
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--ink);
		padding: 0;
		margin-bottom: 0.3rem;
	}
	.hint {
		margin: 0;
		color: var(--text-muted);
		font-size: var(--type-caption-size);
	}
	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 0.4rem;
		margin: 0;
		padding: 0;
		list-style: none;
	}
	/* the chips are tools.css .sc-chip-control; a long name shortens */
	.sc-chip-control {
		max-width: 100%;
	}
	.name {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.icon {
		flex: none;
		width: 1.6rem;
		height: 1.6rem;
		display: inline-flex;
	}
	.icon img {
		width: 100%;
		height: 100%;
		object-fit: contain;
	}
</style>
