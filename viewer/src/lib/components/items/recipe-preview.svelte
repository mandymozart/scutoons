<script lang="ts">
	import { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import type { Entry } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import ItemIcon from './item-icon.svelte';

	let {
		first,
		second,
		oncancel,
		ondone
	}: { first: Entry; second: Entry; oncancel: () => void; ondone: (outputItemId: string) => void } =
		$props();

	const inventory = getInventoryContext();
	const recipe = $derived(inventory.recipesFor(first.item_id, second.item_id)[0]);
	const ready = $derived(recipe ? inventory.canCraft(recipe) : false);
	let error = $state<string | null>(null);
	let crafting = $state(false);

	const part = (item_id: string) => ({ item_id, variant: null });

	async function craft() {
		if (!recipe) return;
		crafting = true;
		const result = await inventory.craft(recipe);
		crafting = false;
		if (result.ok) ondone(recipe.output.item);
		else error = result.error === 'missing_inputs' ? m['items.combine_missing']() : m['claim.failed']();
	}
</script>

<div class="recipe">
	{#if recipe}
		<div class="equation">
			{#each recipe.inputs as input, i}
				{#if i > 0}<span class="op">+</span>{/if}
				<div class="part">
					<ItemIcon srcs={[inventory.item(input.item)?.icon ?? '']} name={inventory.nameOf(part(input.item))} />
					<span>{input.qty > 1 ? `${input.qty}× ` : ''}{inventory.nameOf(part(input.item))}</span>
				</div>
			{/each}
			<span class="op">=</span>
			<div class="part output">
				<ItemIcon
					srcs={[inventory.item(recipe.output.item)?.icon ?? '']}
					name={inventory.nameOf(part(recipe.output.item))}
				/>
				<span>{inventory.nameOf(part(recipe.output.item))}</span>
			</div>
		</div>
		{#if !ready}
			<p class="note">{m['items.combine_missing']()}</p>
		{/if}
		{#if error}
			<p class="note">{error}</p>
		{/if}
		<div class="actions">
			<button class="btn-secondary" onclick={oncancel}>{m['common.cancel']()}</button>
			<button class:disabled={!ready || crafting} disabled={!ready || crafting} onclick={craft}>
				{m['items.craft']()}
			</button>
		</div>
	{:else}
		<p class="note">{m['items.combine_none']()}</p>
		<div class="actions">
			<button class="btn-secondary" onclick={oncancel}>{m['items.back']()}</button>
		</div>
	{/if}
</div>

<style>
	.recipe {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}
	.equation {
		display: flex;
		flex-wrap: wrap;
		justify-content: center;
		align-items: center;
		gap: 0.25rem;
	}
	.part {
		width: 5.5rem;
		box-sizing: border-box;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.25rem;
		font-size: 0.75rem;
		line-height: 1.3;
		text-align: center;
		border: 2px solid #ccc;
		padding: 0.25rem;
	}
	.part.output {
		border-color: black;
		filter: drop-shadow(0 0 6px var(--primary-color));
		background: white;
	}
	.op {
		font-weight: 700;
	}
	.note {
		margin: 0;
		font-size: 0.75rem;
		color: #6b6b6b;
	}
	.actions {
		display: flex;
		gap: 0.5rem;
	}
	.actions button {
		flex: 1;
		padding: 0.75rem 1rem;
	}
</style>
