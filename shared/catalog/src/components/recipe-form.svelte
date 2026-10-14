<script lang="ts">
	import { MediaSlot } from '@scutoons/media';
	import { Button, CloseIcon } from '@scutoons/ui';
	import { problemFor, text, type CatalogFormProps, type Data } from '../kinds.ts';
	import { m } from '../messages.ts';
	import CheckField from './check-field.svelte';

	/**
	 * A recipe (agents/ITEMS.md): the items it takes (consumed, unless kept – a tool like the drill) and the item it makes,
	 * each with a quantity; the output may take its variant (colour) from an input (research/SCORES.md §2a); its own
	 * optional picture (owner 2026-10-04; readers see the output item's picture without one) in the output item's toon folder.
	 */
	let { data = $bindable(), lists, problems }: CatalogFormProps = $props();

	const items = $derived((lists.items ?? []).map((e) => ({ value: e.id, label: `${text(e.data?.name) || e.id} · ${e.id}` })));
	const inputs = $derived<Data[]>(Array.isArray(data.inputs) ? data.inputs : []);
	const qty = (raw: string) => Math.max(1, Math.floor(Number(raw) || 1));
	const err = (...words: string[]) => problemFor(problems, ...words);
	const outputToon = $derived((lists.items ?? []).find((e) => e.id === data.output?.item)?.data?.toon || 'scutoons');
	let bust = $state(0);
</script>

{#snippet part(p: Data, label: string, testid: string, remove?: () => void)}
	<div class="part" data-testid={testid}>
		<select value={p.item ?? ''} onchange={(e) => (p.item = e.currentTarget.value)} aria-label="{label}: {m['catalog.item']()}" data-testid="{testid}-item">
			<option value="">{m['catalog.choose']()}</option>
			{#each items as o (o.value)}<option value={o.value}>{o.label}</option>{/each}
			{#if p.item && !items.some((o) => o.value === p.item)}<option value={p.item}>{p.item} (?)</option>{/if}
		</select>
		<input type="number" min="1" step="1" value={p.qty ?? 1} onchange={(e) => (p.qty = qty(e.currentTarget.value))} aria-label="{label}: {m['catalog.qty']()}" data-testid="{testid}-qty" />
		{#if remove}
			<button type="button" class="sc-round" title={m['catalog.remove']()} aria-label={m['catalog.remove']()} onclick={remove}><CloseIcon /></button>
		{/if}
	</div>
{/snippet}

<div class="group">
	<MediaSlot toon="" value={data.image ?? undefined} label={m['catalog.picture']()} place="{outputToon}/items/recipes/" {bust} onchange={(path) => { if (path) data.image = path; else delete data.image; bust = Date.now(); }} testid="recipe-image" />
	{#if err('image')}<p class="sc-error">{err('image')}</p>{/if}
</div>

<fieldset class="group">
	<legend>{m['catalog.recipe_inputs']()}</legend>
	{#each inputs as input, i (i)}
		{@render part(input, `${m['catalog.recipe_input']()} ${i + 1}`, 'recipe-input', inputs.length > 1 ? () => (data.inputs = inputs.filter((_, j) => j !== i)) : undefined)}
		<CheckField label={m['catalog.recipe_keep']()} checked={input.keep === true} onchange={(on) => (on ? (input.keep = true) : delete input.keep)} testid="recipe-input-keep" />
	{/each}
	<span><Button size="sm" variant="secondary" onclick={() => (data.inputs = [...inputs, { item: '', qty: 1 }])} data-testid="recipe-add-input">{m['catalog.add_input']()}</Button></span>
	{#if err('inputs', 'input')}<p class="sc-error">{err('inputs', 'input')}</p>{/if}
</fieldset>

<fieldset class="group">
	<legend>{m['catalog.recipe_output']()}</legend>
	{#if data.output && typeof data.output === 'object'}
		{@render part(data.output, m['catalog.recipe_output'](), 'recipe-output')}
		<label class="sc-label" for="recipe-variant-from">{m['catalog.recipe_variant_from']()}</label>
		<select id="recipe-variant-from" value={data.output.variant_from ?? ''} onchange={(e) => (e.currentTarget.value ? (data.output.variant_from = e.currentTarget.value) : delete data.output.variant_from)} data-testid="recipe-variant-from">
			<option value="">{m['catalog.none']()}</option>
			{#each inputs.filter((p) => p.item) as p (p.item)}<option value={p.item}>{items.find((o) => o.value === p.item)?.label ?? p.item}</option>{/each}
		</select>
	{:else}
		<span><Button size="sm" variant="secondary" onclick={() => (data.output = { item: '', qty: 1 })}>{m['catalog.add_output']()}</Button></span>
	{/if}
	{#if err('output')}<p class="sc-error">{err('output')}</p>{/if}
</fieldset>

<style>
	.group {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		margin-bottom: var(--space-15);
	}
	.part {
		display: grid;
		grid-template-columns: minmax(0, 1fr) 5rem auto;
		gap: var(--space-05);
		align-items: center;
	}
</style>
