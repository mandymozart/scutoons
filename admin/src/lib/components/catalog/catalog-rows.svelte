<script lang="ts" generics="T">
	import type { Snippet } from 'svelte';
	import { Button, CloseIcon, ReorderButtons } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A list inside a catalog form (an item's / product's variants, an album's tracks): one fieldset, a row per entry
	 * with its head (the id field or the number), optional ↑ / ↓, × remove, the row's own fields, "+ add" at the end.
	 * The form keeps the data: `onchange(next)` gets the new list (moved, removed, added).
	 */
	let {
		legend,
		items,
		onchange,
		create,
		addLabel,
		removeLabel,
		reorder = false,
		error = null,
		testid,
		rowTestid,
		addTestid,
		head,
		row,
		footer
	}: {
		legend: string;
		items: T[];
		onchange: (next: T[]) => void;
		/** a new, empty entry for "+ add" */
		create: () => T;
		addLabel: string;
		removeLabel: string;
		/** ↑ / ↓ per row (an album's track order) */
		reorder?: boolean;
		error?: string | null;
		testid?: string;
		rowTestid?: string;
		addTestid?: string;
		/** the row's head (default: its number) */
		head?: Snippet<[T, number]>;
		row: Snippet<[T, number]>;
		/** under the list (a hint) */
		footer?: Snippet;
	} = $props();

	function move(from: number, to: number) {
		const next = [...items];
		const [entry] = next.splice(from, 1);
		next.splice(to, 0, entry);
		onchange(next);
	}
</script>

<fieldset class="rows" data-testid={testid}>
	<legend>{legend}</legend>
	{#if error}<p class="sc-error">{error}</p>{/if}
	{#each items as entry, i (i)}
		<div class="row" data-testid={rowTestid}>
			<div class="head">
				<span class="lead">{#if head}{@render head(entry, i)}{:else}<span class="sc-caption">{i + 1}</span>{/if}</span>
				<span class="tools">
					{#if reorder}<ReorderButtons index={i} count={items.length} onmove={move} variant="sc-round" labels={{ up: m['catalog.up'](), down: m['catalog.down']() }} />{/if}
					<button type="button" class="sc-round" onclick={() => onchange(items.filter((_, j) => j !== i))} title={removeLabel} aria-label={removeLabel}><CloseIcon /></button>
				</span>
			</div>
			{@render row(entry, i)}
		</div>
	{/each}
	<span><Button size="sm" variant="secondary" onclick={() => onchange([...items, create()])} data-testid={addTestid}>{addLabel}</Button></span>
	{@render footer?.()}
</fieldset>

<style>
	.rows {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		margin-bottom: var(--space-15);
	}
	.row {
		display: flex;
		flex-direction: column;
		padding-bottom: var(--space-1);
		border-bottom: var(--border-width-hairline) solid var(--border-subtle, currentColor);
	}
	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-05);
		margin-bottom: var(--space-05);
	}
	.head:has(.lead :global(.field)) {
		align-items: flex-end;
		margin-bottom: 0;
	}
	.lead {
		flex: 1;
		min-width: 0;
	}
	.tools {
		display: inline-flex;
		gap: var(--space-025);
		flex: none;
	}
	/* an id field in the head: the buttons on its input's line (Field keeps its bottom margin) */
	.head:has(.lead :global(.field)) .tools {
		margin-bottom: var(--space-15);
	}
	p {
		margin: 0;
	}
</style>
