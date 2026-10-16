<script lang="ts">
	import { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import type { Entry } from '@scutoons/content';
	import { StickyActions } from '@scutoons/ui';
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
	// the same two items can make more than one thing (camera + testo → testo camera or Check It camera): pick one
	const recipes = $derived(inventory.recipesFor(first.item_id, second.item_id));
	let pick = $state(0);
	const recipe = $derived(recipes[Math.min(pick, Math.max(0, recipes.length - 1))]);
	const ready = $derived(recipe ? inventory.canCraft(recipe) : false);
	let error = $state<string | null>(null);
	let crafting = $state(false);

	const part = (item_id: string) => ({ item_id, variant: null });

	async function craft() {
		if (!recipe) return;
		crafting = true;
		const result = await inventory.craft(recipe, { [first.item_id]: first.id, [second.item_id]: second.id });
		crafting = false;
		if (result.ok) ondone(recipe.output.item);
		else error = result.error === 'missing_inputs' ? m['items.combine_missing']() : m['claim.failed']();
	}
</script>

<div class="recipe">
	{#if recipe}
		{#if recipes.length > 1}
			<div class="choices" role="group" aria-label={m['items.combine_choose']()}>
				{#each recipes as r, i (r.id)}
					<button class={i === pick ? '' : 'btn-secondary'} aria-pressed={i === pick} data-recipe={r.id} onclick={() => ((pick = i), (error = null))}>
						→ {inventory.nameOf(part(r.output.item))}
					</button>
				{/each}
			</div>
		{/if}
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
					srcs={[recipe.image ?? '', inventory.item(recipe.output.item)?.icon ?? '']}
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
		<!-- stuck to the panel's bottom like the item's actions (StickyActions, DESIGN.md "Sticky action bar") -->
		<StickyActions testid="recipe-actions">
			<button class="btn-secondary" onclick={oncancel}>{m['common.cancel']()}</button>
			<button class:disabled={!ready || crafting} disabled={!ready || crafting} onclick={craft}>
				{m['items.craft']()}
			</button>
		</StickyActions>
	{:else}
		<p class="note">{m['items.combine_none']()}</p>
		<StickyActions testid="recipe-actions">
			<button class="btn-secondary" onclick={oncancel}>{m['items.back']()}</button>
		</StickyActions>
	{/if}
</div>

<style>
	.recipe {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}
	.choices {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}
	.choices button {
		flex: 1 1 auto;
		padding: 0 0.75rem;
		font-size: 0.75rem;
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
		border: 2px solid var(--border-muted);
		padding: 0.25rem;
	}
	.part.output {
		border-color: var(--ink);
		filter: drop-shadow(0 0 6px var(--primary-color));
		background: var(--paper);
	}
	.op {
		font-weight: 700;
	}
	.note {
		margin: 0;
		font-size: 0.75rem;
		color: var(--text-muted);
	}
	.recipe :global(.sc-sticky-actions button) {
		flex: 1;
		padding: 0 1rem;
	}
</style>
