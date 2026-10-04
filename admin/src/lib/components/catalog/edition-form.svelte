<script lang="ts">
	import { Button, CloseIcon } from '@scutoons/ui';
	import { listText, parseList, problemFor, text, type CatalogFormProps, type Data } from '$lib/catalog/kinds.ts';
	import { m } from '$lib/paraglide/messages.js';
	import CatalogField from './catalog-field.svelte';

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
<div class="pair">
	<CatalogField label={m['catalog.edition_number']()} type="number" value={data.edition_number} error={err('edition_number')} onchange={(v) => (data.edition_number = whole(v))} />
	<CatalogField label={m['catalog.edition_count']()} type="number" value={data.edition_count} error={err('edition_count')} onchange={(v) => (data.edition_count = whole(v))} />
	<CatalogField label={m['batches.excluded']()} type="number" value={data.excluded_number} error={err('excluded_number')} onchange={(v) => (data.excluded_number = whole(v))} />
</div>
<CatalogField label={m['catalog.created_at']()} value={data.created_at} placeholder="2026-10-04T20:00:00.000Z" onchange={(v) => (v ? (data.created_at = v) : delete data.created_at)} />
<CatalogField label={m['catalog.creators']()} type="textarea" value={data.creators} onchange={(v) => (data.creators = v)} />

<fieldset class="variants">
	<legend>{m['catalog.variants']()}</legend>
	{#if err('variant', 'variants')}<p class="error">{err('variant', 'variants')}</p>{/if}
	{#each variants as v, i (i)}
		<div class="variant">
			<div class="variant-head">
				<CatalogField label="id" value={v.id} onchange={(x) => (v.id = x)} />
				<button type="button" class="sc-round" title={m['catalog.remove_variant']()} aria-label={m['catalog.remove_variant']()} onclick={() => (data.variants = variants.filter((_, j) => j !== i))}><CloseIcon /></button>
			</div>
			<CatalogField label={m['catalog.name']()} value={v.name} onchange={(x) => (v.name = x)} />
			<div class="pair">
				<CatalogField label={m['catalog.sizes']()} value={listText(v.sizes)} hint={m['catalog.list_hint']()} onchange={(x) => (v.sizes = parseList(x))} />
				<CatalogField label={m['catalog.colors']()} value={listText(v.colors)} hint={m['catalog.list_hint']()} onchange={(x) => (v.colors = parseList(x))} />
			</div>
		</div>
	{/each}
	<span><Button size="sm" variant="secondary" onclick={() => (data.variants = [...variants, { id: '', name: '', sizes: [], colors: [] }])}>{m['catalog.add_variant']()}</Button></span>
</fieldset>

{#if !creating && id}
	<p class="links">
		<a href="/codes?edition_id={encodeURIComponent(id)}">{m['nav.codes']()}</a>
		<a href="/batches?edition_id={encodeURIComponent(id)}">{m['nav.batches']()}</a>
	</p>
{/if}

<style>
	.pair {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(8rem, 1fr));
		gap: 0 var(--space-1);
	}
	.variants {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		margin-bottom: var(--space-15);
	}
	.variant {
		padding-bottom: var(--space-1);
		border-bottom: 1px solid var(--border-subtle, currentColor);
	}
	.variant-head {
		display: flex;
		align-items: flex-end;
		gap: var(--space-05);
	}
	.variant-head :global(.field) {
		flex: 1;
	}
	.variant-head .sc-round {
		margin-bottom: var(--space-15);
	}
	.links {
		display: flex;
		gap: var(--space-1);
		margin: 0;
	}
	.error {
		margin: 0;
		color: var(--state-error);
	}
</style>
