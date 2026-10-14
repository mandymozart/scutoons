<script lang="ts">
	import { Button } from '@scutoons/ui';
	import {
		fill,
		type HomeLabels,
		type HomePiece,
		type HomeRecipe,
		type HomeThing
	} from './home-types.ts';

	/**
	 * The panel of the home view (research/CAVE.md D4–D6): the opened piece – a rack's contents (tap one to take it out),
	 * the workbench's recipes, "Turn" – then what the reader carries (tap to select; then tap a rack or the room).
	 */
	let {
		open,
		carried,
		recipes,
		selected,
		visitor,
		note,
		labels,
		onselect,
		ontakeout,
		onturn,
		oncraft
	}: {
		open: HomePiece | null;
		carried: HomeThing[];
		recipes: HomeRecipe[];
		selected: string | null;
		visitor: boolean;
		note: string;
		labels: HomeLabels;
		onselect: (thing: HomeThing | null) => void;
		ontakeout: (thing: HomeThing, piece: HomePiece) => void;
		onturn: (piece: HomePiece) => void;
		oncraft: (recipe: HomeRecipe) => void;
	} = $props();

	const placing = $derived(carried.find((c) => c.id === selected)?.furniture === true);
</script>

<aside class="panel" data-testid="home-panel">
	{#if note}<p class="note" role="status">{note}</p>{/if}
	{#if open}
		<h2>
			{open.name}{#if open.holds}<span class="muted"
					>{` · ${fill(labels.count, { count: open.contents.length, max: open.holds.max ?? '∞' })}`}</span
				>{/if}
		</h2>
		{#if open.holds}
			<div class="grid">
				{#each open.contents as thing (thing.id)}
					<button
						class="thing"
						disabled={visitor}
						title={thing.name}
						onclick={() => ontakeout(thing, open)}
						data-testid="home-content"
					>
						<img src={thing.icon} alt="" /><span>{thing.name}</span>
					</button>
				{/each}
			</div>
			{#if !visitor}<p class="muted">{fill(labels.putInHint, { name: open.name })}</p>{/if}
		{/if}
		{#if open.workshop}
			<h3>{labels.workshop}</h3>
			{#each recipes as recipe (recipe.id)}
				<Button
					variant="secondary"
					disabled={!recipe.possible}
					onclick={() => oncraft(recipe)}
					data-testid="home-recipe">{recipe.name}</Button
				>
			{:else}
				<p class="muted">{labels.noRecipes}</p>
			{/each}
		{/if}
		{#if !visitor}<Button variant="secondary" onclick={() => onturn(open)} data-testid="home-turn"
				>{labels.turn}</Button
			>{/if}
	{/if}

	<h2>{visitor ? labels.visiting : labels.inventory}</h2>
	<div class="grid">
		{#each carried as thing (thing.id)}
			<button
				class="thing"
				class:selected={selected === thing.id}
				disabled={visitor}
				title={thing.name}
				onclick={() => onselect(selected === thing.id ? null : thing)}
				data-testid="home-carried"
			>
				<img src={thing.icon} alt="" /><span>{thing.name}</span>
			</button>
		{/each}
	</div>
	{#if placing}<p class="muted">{labels.placeHint}</p>{/if}
</aside>

<style>
	.panel {
		display: grid;
		gap: 0.5rem;
		align-content: start;
		overflow-y: auto;
		padding: 0 1rem 1rem;
	}
	h2,
	h3 {
		margin: 0.5rem 0 0;
		font-size: 1rem;
	}
	p {
		margin: 0;
	}
	.muted {
		color: var(--muted);
	}
	.note {
		padding: 0.5rem;
		border: var(--border-width-regular) solid var(--ink);
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(5.5rem, 1fr));
		gap: 0.5rem;
	}
	.thing {
		display: grid;
		justify-items: center;
		gap: 0.25rem;
		padding: 0.25rem;
		border: var(--border-width-regular) solid transparent;
		background: none;
		color: var(--ink);
		font: inherit;
		font-size: 0.75rem;
		cursor: pointer;
	}
	.thing img {
		width: 3.5rem;
		height: 3.5rem;
		object-fit: contain;
	}
	.thing.selected {
		border-color: var(--ink);
	}
	.thing:disabled {
		cursor: default;
	}
	.panel > :global(.sc-button) {
		justify-self: start;
	}
</style>
