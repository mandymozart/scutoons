<script lang="ts">
	import { Button, CloseIcon } from '@scutoons/ui';
	import { problemFor, text, type CatalogFormProps, type Data } from '$lib/catalog/kinds.ts';
	import { m } from '$lib/paraglide/messages.js';

	/** A recipe (agents/ITEMS.md): the items it takes (all consumed) and the item it makes, each with a quantity */
	let { data = $bindable(), lists, problems }: CatalogFormProps = $props();

	const items = $derived((lists.items ?? []).map((e) => ({ value: e.id, label: `${text(e.data?.name) || e.id} · ${e.id}` })));
	const inputs = $derived<Data[]>(Array.isArray(data.inputs) ? data.inputs : []);
	const qty = (raw: string) => Math.max(1, Math.floor(Number(raw) || 1));
	const err = (...words: string[]) => problemFor(problems, ...words);
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

<fieldset class="group">
	<legend>{m['catalog.recipe_inputs']()}</legend>
	{#each inputs as input, i (i)}
		{@render part(input, `${m['catalog.recipe_input']()} ${i + 1}`, 'recipe-input', inputs.length > 1 ? () => (data.inputs = inputs.filter((_, j) => j !== i)) : undefined)}
	{/each}
	<span><Button size="sm" variant="secondary" onclick={() => (data.inputs = [...inputs, { item: '', qty: 1 }])} data-testid="recipe-add-input">{m['catalog.add_input']()}</Button></span>
	{#if err('inputs', 'input')}<p class="sc-error">{err('inputs', 'input')}</p>{/if}
</fieldset>

<fieldset class="group">
	<legend>{m['catalog.recipe_output']()}</legend>
	{#if data.output && typeof data.output === 'object'}
		{@render part(data.output, m['catalog.recipe_output'](), 'recipe-output')}
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
