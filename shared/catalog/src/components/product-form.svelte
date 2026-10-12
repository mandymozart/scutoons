<script lang="ts">
	import { MediaSlot } from '@scutoons/media';
	import { listText, parseList, problemFor, text, type CatalogFormProps, type Data } from '../kinds.ts';
	import { m } from '../messages.ts';
	import CatalogField from './catalog-field.svelte';
	import CatalogRows from './catalog-rows.svelte';
	import CheckField from './check-field.svelte';
	import LocaleText from './locale-text.svelte';

	/**
	 * A shop product (`Product` in @scutoons/content, the old config/products.json entry): the digital item it belongs
	 * to, its edition, shop facts (link, price, limited / remaining, sizes, credits), the picture and the colour
	 * variants with their pictures (icon + the shop's image gallery).
	 */
	let { data = $bindable(), lists, problems }: CatalogFormProps = $props();

	const options = (kind: 'toons' | 'editions' | 'items') => (lists[kind] ?? []).map((e) => ({ value: e.id, label: `${text(e.data?.name) || e.id}` }));
	const variants = $derived<Data[]>(Array.isArray(data.variants) ? data.variants : []);
	const sizes = $derived<string[]>(Array.isArray(data.sizes) ? data.sizes : []);
	const soldOut = $derived<string[]>(Array.isArray(data.soldOutSizes) ? data.soldOutSizes : []);
	const iconPlace = $derived(`${data.toon || 'scutoons'}/items/`);
	const imagePlace = $derived(`${data.toon || 'scutoons'}/merch/`);
	const err = (...words: string[]) => problemFor(problems, ...words);

	function toggleSoldOut(size: string, on: boolean) {
		data.soldOutSizes = on ? [...soldOut.filter((s) => s !== size), size] : soldOut.filter((s) => s !== size);
	}
	function setImage(v: Data, i: number, path: string | undefined) {
		const images: string[] = Array.isArray(v.images) ? [...v.images] : [];
		if (path) images[i] = path;
		else images.splice(i, 1);
		v.images = images;
	}
</script>

<CatalogField label={m['catalog.toon']()} type="select" value={data.toon} options={options('toons')} error={err('toon')} onchange={(v) => (data.toon = v)} />
<CatalogField label={m['catalog.name']()} required value={typeof data.name === 'string' ? data.name : text(data.name)} error={err('name')} onchange={(v) => (data.name = v)} testid="product-name" />
<CatalogField label={m['catalog.item']()} type="select" value={data.item_id} options={options('items')} none={m['catalog.none']()} hint={m['catalog.product_item_hint']()} error={err('item', 'item_id')} onchange={(v) => (data.item_id = v)} />
<CatalogField label={m['common.edition']()} type="select" value={data.edition_id} options={options('editions')} none={m['catalog.none']()} error={err('edition', 'edition_id')} onchange={(v) => (data.edition_id = v)} />
<div class="sc-fields" data-cols="fit" style:--fields-fit-min="8rem">
	<CatalogField label={m['catalog.category']()} value={data.category} onchange={(v) => (data.category = v)} />
	<CatalogField label={m['catalog.edition_size']()} type="number" value={data.edition_size} onchange={(v) => (data.edition_size = v)} />
</div>
<CatalogField label={m['catalog.link']()} type="url" value={data.link} onchange={(v) => (data.link = v)} />
<div class="sc-fields" data-cols="fit" style:--fields-fit-min="8rem">
	<CatalogField label={m['catalog.price']()} type="number" value={data.price} error={err('price')} onchange={(v) => (data.price = v)} />
	<CatalogField label={m['catalog.limited']()} type="number" value={data.limited} onchange={(v) => (data.limited = v)} />
	<CatalogField label={m['catalog.remaining']()} type="number" value={data.remaining} onchange={(v) => (data.remaining = v)} />
</div>
<CatalogField label={m['catalog.sizes']()} value={listText(data.sizes)} hint={m['catalog.list_hint']()} onchange={(v) => (data.sizes = parseList(v))} />
{#if sizes.length}
	<fieldset class="inline">
		<legend>{m['catalog.sold_out_sizes']()}</legend>
		{#each sizes as size (size)}<CheckField label={size} checked={soldOut.includes(size)} onchange={(on) => toggleSoldOut(size, on)} />{/each}
	</fieldset>
{/if}
<CatalogField label={m['catalog.credits']()} type="textarea" value={(data.credits ?? []).join('\n')} hint={m['catalog.lines_hint']()} onchange={(v) => (data.credits = String(v).split('\n').map((s) => s.trim()).filter(Boolean))} />
<div class="picture">
	<MediaSlot toon="" value={data.icon ?? undefined} label={m['catalog.picture']()} place={iconPlace} onchange={(path) => (data.icon = path ?? null)} testid="product-icon" />
</div>

<CatalogRows
	legend={m['catalog.variants']()}
	items={variants}
	onchange={(next) => (data.variants = next)}
	create={() => ({ id: '', name: { de: '', en: '' }, soldOut: false, icon: null, images: [] })}
	addLabel={m['catalog.add_variant']()}
	removeLabel={m['catalog.remove_variant']()}
	error={err('variant', 'variants', 'defaultVariant')}
	testid="product-variants"
>
	{#snippet head(v)}<CatalogField label="id" value={v.id} onchange={(x) => (v.id = x)} />{/snippet}
	{#snippet row(v)}
		<LocaleText label={m['catalog.name']()} value={v.name} onchange={(x) => (v.name = x)} />
		<CheckField label={m['catalog.sold_out']()} checked={!!v.soldOut} onchange={(x) => (v.soldOut = x)} />
		<div class="slots">
			<MediaSlot toon="" value={v.icon ?? undefined} label={m['catalog.picture']()} place={iconPlace} onchange={(path) => (v.icon = path ?? null)} />
			{#each [...(Array.isArray(v.images) ? v.images : []), undefined] as image, j (j)}
				<MediaSlot toon="" value={image} label="{m['catalog.image']()} {j + 1}" place={imagePlace} onchange={(path) => setImage(v, j, path)} />
			{/each}
		</div>
	{/snippet}
</CatalogRows>
<CatalogField
	label={m['catalog.default_variant']()}
	type="select"
	value={data.defaultVariant}
	options={variants.filter((v) => v.id).map((v) => ({ value: v.id, label: text(v.name) || v.id }))}
	none={m['catalog.none']()}
	onchange={(v) => (data.defaultVariant = v)}
/>

<style>
	.inline {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05) var(--space-1);
		margin-bottom: var(--space-15);
	}
	.picture {
		margin-bottom: var(--space-15);
	}
	.slots {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-1);
	}
</style>
