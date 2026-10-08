<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { GIVE_ITEM_MAX, type Path, type Repeat } from '@scutoons/content';
	import ItemPickerSheet from '$lib/components/items/item-picker-sheet.svelte';
	import PropField from '$lib/components/fields/prop-field.svelte';
	import RepeatsField from '$lib/components/fields/repeats-field.svelte';
	import { itemCatalog, itemName, loadCatalog } from '$lib/editor/catalog.svelte.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * What a script gives (`giveItem` / a choice's `give` – BACKLOG 10.38) or, with `take`, takes (`takeItem` / `take`):
	 * a catalog item, picked from the catalog (a script can't make items, and a typo can't become a variable), and how
	 * many. Taking: only items the catalog marks takeable (never physical ones) are offered, plus how often it takes
	 * again (the trigger intervals). `path` = the object with `item` / `qty` (/ `repeat`).
	 */
	let { doc, path, take = false, testid = 'give-item' }: { doc: ContentDoc; path: Path; take?: boolean; testid?: string } = $props();

	$effect(() => {
		loadCatalog();
	});
	const id = $derived(doc.get(doc.primary, [...path, 'item']) as string | undefined);
	const def = $derived(itemCatalog.catalog?.items.find((i) => i.id === id));
	/** taking: only what a story may take is offered (the catalog flag, never a physical item) */
	const takeable = (i: { kind: string; takeable?: boolean }) => i.kind !== 'physical' && i.takeable === true;
	const repeat = $derived(doc.get(doc.primary, [...path, 'repeat']) as Repeat | undefined);
	let picking = $state(false);
	/** giving: one of the item's variants (a colour of the sunglasses – owner 2026-10-08); none = the item itself */
	const variants = $derived(take ? [] : (def?.variants ?? []));
	const variantOptions = $derived(variants.map((v) => ({ value: v.id, label: localName(v.name) || v.id })));
	const localName = (name: unknown) => (typeof name === 'string' ? name : ((name as Record<string, string> | null)?.de ?? (name as Record<string, string> | null)?.en ?? ''));
	const variantId = $derived(doc.get(doc.primary, [...path, 'variant']) as string | undefined);
	const shownIcon = $derived(variants.find((v) => v.id === variantId)?.icon || def?.icon);
</script>

<div class="field">
	<span class="sc-label">{m['creator.field_item']()}</span>
	<button type="button" class="secondary pick" onclick={() => (picking = true)} data-testid="{testid}-pick">
		{#if shownIcon}<img src={asset(shownIcon)} alt="" />{/if}
		<span>{id ? itemName(def, id) : m['creator.slice_item_change']()}</span>
	</button>
</div>
<PropField {doc} path={[...path, 'qty']} label={m['creator.field_qty']()} type="number" step={1} min={1} max={GIVE_ITEM_MAX} placeholder="1" />
{#if variants.length}
	<PropField {doc} path={[...path, 'variant']} label={m['creator.item_variant']()} type="select" options={variantOptions} placeholder={m['creator.item_variant_none']()} />
{/if}
{#if take}
	<RepeatsField id="{testid}-repeat" testid="{testid}-repeat" value={repeat} onchange={(next) => doc.set([...path, 'repeat'], next)} />
{/if}
<ItemPickerSheet bind:open={picking} title={m['creator.slice_item_change']()} only={take ? takeable : undefined} onpick={(item) => (doc.set([...path, 'item'], item.id), doc.set([...path, 'variant'], undefined))} />

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
