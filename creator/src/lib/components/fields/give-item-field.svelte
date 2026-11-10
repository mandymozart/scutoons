<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { AddressEntry } from '@scutoons/api';
	import { GIVE_ITEM_MAX, type Path, type Repeat } from '@scutoons/content';
	import PropField from '$lib/components/fields/prop-field.svelte';
	import RepeatsField from '$lib/components/fields/repeats-field.svelte';
	import AddressField from '$lib/components/fields/address-field.svelte';
	import { itemCatalog, loadCatalog } from '$lib/editor/catalog.svelte.ts';
	import { TARGETS } from '$lib/editor/address-targets.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * What a script gives (`giveItem` / a choice's `give` – BACKLOG 10.38) or, with `take`, takes (`takeItem` / `take`):
	 * a catalog item, ONE typeahead over the toon's address book (#477; a script can't make items, and a typo can't become
	 * a variable), and how many. Taking: only items the catalog marks takeable (never physical ones) are offered, plus how
	 * often it takes again (the trigger intervals). `path` = the object with `item` / `qty` (/ `repeat`).
	 */
	let { doc, path, take = false, testid = 'give-item' }: { doc: ContentDoc; path: Path; take?: boolean; testid?: string } = $props();

	$effect(() => {
		loadCatalog();
	});
	const id = $derived(doc.get(doc.primary, [...path, 'item']) as string | undefined);
	const def = $derived(itemCatalog.catalog?.items.find((i) => i.id === id));
	/** taking: only what a story may take is offered (the catalog flag, never a physical item) */
	const takeable = (i: { kind: string; takeable?: boolean }) => i.kind !== 'physical' && i.takeable === true;
	const offered = $derived(take ? (e: AddressEntry) => !!itemCatalog.catalog?.items.some((i) => i.id === e.name && takeable(i)) : undefined);
	const repeat = $derived(doc.get(doc.primary, [...path, 'repeat']) as Repeat | undefined);
	/** giving: one of the item's variants (a colour of the sunglasses – owner 2026-10-08); none = the item itself */
	const variants = $derived(take ? [] : (def?.variants ?? []));
	const variantOptions = $derived(variants.map((v) => ({ value: v.id, label: localName(v.name) || v.id })));
	const localName = (name: unknown) => (typeof name === 'string' ? name : ((name as Record<string, string> | null)?.de ?? (name as Record<string, string> | null)?.en ?? ''));
	/** the item's picture in the field's row (the variant's when one is given) and on each option (#477 slice D) */
	const variantId = $derived(doc.get(doc.primary, [...path, 'variant']) as string | undefined);
	const shownIcon = $derived(variants.find((v) => v.id === variantId)?.icon || def?.icon);
	const iconOf = (e: AddressEntry) => itemCatalog.catalog?.items.find((i) => i.id === e.name)?.icon || undefined;
</script>

<!-- another item: the old one's variant goes -->
<AddressField {doc} {path} spec={TARGETS.item} accept={offered} onpick={(t) => delete t.variant} icon={iconOf} label={m['creator.field_item']()} testid="{testid}-item">
	{#snippet lead()}
		{#if shownIcon}<img class="icon" src={asset(shownIcon)} alt="" />{/if}
	{/snippet}
</AddressField>
<PropField {doc} path={[...path, 'qty']} label={m['creator.field_qty']()} type="number" step={1} min={1} max={GIVE_ITEM_MAX} placeholder="1" />
{#if variants.length}
	<PropField {doc} path={[...path, 'variant']} label={m['creator.item_variant']()} type="select" options={variantOptions} placeholder={m['creator.item_variant_none']()} />
{/if}
{#if take}
	<RepeatsField id="{testid}-repeat" testid="{testid}-repeat" value={repeat} onchange={(next) => doc.set([...path, 'repeat'], next)} />
{/if}

<style>
	.icon {
		flex: none;
		width: 1.5rem;
		height: 1.5rem;
		object-fit: contain;
	}
</style>
