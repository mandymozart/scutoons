<script lang="ts">
	import { listText, parseList, problemFor, text, type CatalogFormProps, type Data } from '../kinds.ts';
	import { m } from '../messages.ts';
	import { adminHref } from '../setup.ts';
	import CatalogField from './catalog-field.svelte';
	import CatalogRows from './catalog-rows.svelte';

	/** An edition (the QR codes' editions: batches, certificates): name, numbers, creators and its variants (sizes / colours) */
	let { data = $bindable(), id, creating, lists, problems }: CatalogFormProps = $props();

	const toons = $derived((lists.toons ?? []).map((e) => ({ value: e.id, label: text(e.data?.name) || e.id })));
	const variants = $derived<Data[]>(Array.isArray(data.variants) ? data.variants : []);
	const err = (...words: string[]) => problemFor(problems, ...words);
	const whole = (v: number | null) => (v === null ? null : Math.max(0, Math.floor(v)));
</script>

<CatalogField label={m['catalog.toon']()} type="select" value={data.toon} options={toons} error={err('toon')} onchange={(v) => (data.toon = v)} />
<CatalogField label={m['catalog.name']()} required value={data.name} error={err('name')} onchange={(v) => (data.name = v)} testid="edition-name" />
<CatalogField label={m['catalog.description']()} type="textarea" value={data.description} onchange={(v) => (data.description = v)} />
<div class="sc-fields" data-cols="fit" style:--fields-fit-min="8rem">
	<CatalogField label={m['catalog.edition_number']()} type="number" value={data.edition_number} error={err('edition_number')} onchange={(v) => (data.edition_number = whole(v))} />
	<CatalogField label={m['catalog.edition_count']()} type="number" value={data.edition_count} error={err('edition_count')} onchange={(v) => (data.edition_count = whole(v))} />
	<CatalogField label={m['batches.excluded']()} type="number" value={data.excluded_number} error={err('excluded_number')} onchange={(v) => (data.excluded_number = whole(v))} />
</div>
<CatalogField label={m['catalog.created_at']()} value={data.created_at} placeholder="2026-10-04T20:00:00.000Z" onchange={(v) => (v ? (data.created_at = v) : delete data.created_at)} />
<CatalogField label={m['catalog.creators']()} type="textarea" value={data.creators} onchange={(v) => (data.creators = v)} />

<CatalogRows
	legend={m['catalog.variants']()}
	items={variants}
	onchange={(next) => (data.variants = next)}
	create={() => ({ id: '', name: '', sizes: [], colors: [] })}
	addLabel={m['catalog.add_variant']()}
	removeLabel={m['catalog.remove_variant']()}
	error={err('variant', 'variants')}
>
	{#snippet head(v)}<CatalogField label="id" value={v.id} onchange={(x) => (v.id = x)} />{/snippet}
	{#snippet row(v)}
		<CatalogField label={m['catalog.name']()} value={v.name} onchange={(x) => (v.name = x)} />
		<div class="sc-fields" data-cols="fit" style:--fields-fit-min="8rem">
			<CatalogField label={m['catalog.sizes']()} value={listText(v.sizes)} hint={m['catalog.list_hint']()} onchange={(x) => (v.sizes = parseList(x))} />
			<CatalogField label={m['catalog.colors']()} value={listText(v.colors)} hint={m['catalog.list_hint']()} onchange={(x) => (v.colors = parseList(x))} />
		</div>
	{/snippet}
</CatalogRows>

{#if !creating && id}
	<p class="links">
		<a href={adminHref(`/codes?edition_id=${encodeURIComponent(id)}`)}>{m['nav.codes']()}</a>
		<a href={adminHref(`/batches?edition_id=${encodeURIComponent(id)}`)}>{m['nav.batches']()}</a>
	</p>
{/if}

<style>
	.links {
		display: flex;
		gap: var(--space-1);
		margin: 0;
	}
</style>
