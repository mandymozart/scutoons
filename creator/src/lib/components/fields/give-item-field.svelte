<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { GIVE_ITEM_MAX, type Path } from '@scutoons/content';
	import ItemPickerSheet from '$lib/components/items/item-picker-sheet.svelte';
	import PropField from '$lib/components/fields/prop-field.svelte';
	import { itemCatalog, itemName, loadCatalog } from '$lib/editor/catalog.svelte.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * What a script gives (`giveItem` / a choice's `give` – BACKLOG 10.38): a catalog item, picked from the catalog (a
	 * script can't make items, and a typo can't become a variable), and how many. `path` = the object with `item` / `qty`.
	 */
	let { doc, path, testid = 'give-item' }: { doc: ContentDoc; path: Path; testid?: string } = $props();

	$effect(() => {
		loadCatalog();
	});
	const id = $derived(doc.get(doc.primary, [...path, 'item']) as string | undefined);
	const def = $derived(itemCatalog.catalog?.items.find((i) => i.id === id));
	let picking = $state(false);
</script>

<div class="field">
	<span class="sc-label">{m['creator.field_item']()}</span>
	<button type="button" class="secondary pick" onclick={() => (picking = true)} data-testid="{testid}-pick">
		{#if def?.icon}<img src={asset(def.icon)} alt="" />{/if}
		<span>{id ? itemName(def, id) : m['creator.slice_item_change']()}</span>
	</button>
</div>
<PropField {doc} path={[...path, 'qty']} label={m['creator.field_qty']()} type="number" step={1} min={1} max={GIVE_ITEM_MAX} placeholder="1" />
<ItemPickerSheet bind:open={picking} title={m['creator.slice_item_change']()} onpick={(item) => doc.set([...path, 'item'], item.id)} />

<style>
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		min-width: 0;
	}
	.pick {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		min-width: 0;
	}
	.pick span {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.pick img {
		width: 1.5rem;
		height: 1.5rem;
		object-fit: contain;
		flex: none;
	}
</style>
