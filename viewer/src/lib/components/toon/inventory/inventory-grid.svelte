<script lang="ts">
	import type { Entry } from '@scutoons/content';
	import { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import ItemIcon from '$lib/components/items/item-icon.svelte';
	import { holdCount } from '$lib/components/items/hold-count.ts';
	import { m } from '$lib/paraglide/messages.js';
	import type { Tile } from './pack-mode.svelte.ts';

	/**
	 * The inventory's item grid (moved out of inventory.svelte for the pack mode, research/PACKING.md §2): a tile per entry;
	 * in a mode (`tile`) lit / dimmed with the reason under it, a hold counts the amount (the count on the tile, read out);
	 * dragging onto a container on desktop (`dragTargets` – the same containers the pack mode lights).
	 */
	let {
		listed,
		tile,
		source = null,
		draggable = false,
		onpick,
		onhold,
		oncount,
		ondropinto
	}: {
		listed: { entry: Entry; qty: number }[];
		/** the tile's look in a mode (pack mode, container view), null = a normal tile */
		tile: (entry: Entry) => Tile | null;
		/** combining: the first item */
		source?: Entry | null;
		draggable?: boolean;
		onpick: (entry: Entry) => void;
		onhold: (entry: Entry, qty: number) => void;
		oncount: (count: number) => void;
		ondropinto: (entry: Entry, container: Entry) => void;
	} = $props();

	const inventory = getInventoryContext();
	let dragged = $state<Entry | null>(null);
	let counting = $state<{ id: string; n: number } | null>(null);

	const same = (a: Entry | null | undefined, b: Entry) => !!a && String(a.id) === String(b.id);
	const canDrop = (container: Entry) => !!dragged && inventory.containersFor(dragged).some((c) => same(c, container));

	function iconsFor(entry: Entry): string[] {
		return [inventory.iconOf(entry), inventory.item(entry.item_id)?.icon ?? ''];
	}
	/** a container's fill: "3 / 25" */
	function fill(entry: Entry): string | null {
		const holds = inventory.item(entry.item_id)?.holds;
		if (!holds) return null;
		const n = inventory.countIn(entry);
		return holds.max != null ? `${n} / ${holds.max}` : String(n);
	}
	function drop(container: Entry) {
		const entry = dragged;
		dragged = null;
		if (entry && !same(entry, container)) ondropinto(entry, container);
	}
</script>

<section class="items" data-testid="inventory-items">
	{#each listed as { entry, qty } (entry.id)}
		{@const t = tile(entry)}
		{@const id = String(entry.id)}
		<button
			class="item"
			class:source={t?.look === 'source' || same(source, entry)}
			class:lit={t?.look === 'lit'}
			class:dim={t?.look === 'dim'}
			data-control-exempt="inventory item card (picture + name)"
			data-entry={id}
			data-item={entry.item_id}
			aria-label={t?.label}
			aria-disabled={t?.look === 'dim' || undefined}
			title={t?.reason ?? inventory.nameOf(entry)}
			onclick={() => onpick(entry)}
			use:holdCount={{
				max: t?.holdMax ?? 0,
				onstep: (n) => ((counting = { id, n }), oncount(n)),
				onhold: (n) => ((counting = null), onhold(entry, n)),
				onend: () => (counting = null)
			}}
			{draggable}
			ondragstart={() => (dragged = entry)}
			ondragend={() => (dragged = null)}
			ondragover={(e) => canDrop(entry) && e.preventDefault()}
			ondrop={(e) => (e.preventDefault(), drop(entry))}
			class:drop-target={canDrop(entry)}
		>
			<ItemIcon srcs={iconsFor(entry)} name={inventory.nameOf(entry)} lettering={inventory.letteringOf(entry)} />
			{#if counting?.id === id}
				<div class="count counting" aria-hidden="true" data-testid="hold-count">{m['items.pack_counting']({ count: counting.n })}</div>
			{:else if qty > 1}
				<div class="count">{qty}</div>
			{:else if fill(entry)}
				<div class="count fill" data-testid="container-fill">{fill(entry)}</div>
			{/if}
			<span class="name" class:why={!!t?.reason}>{t?.reason ?? inventory.nameOf(entry)}</span>
		</button>
	{/each}
</section>

<style>
	.items {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(5rem, 1fr));
		gap: 0.5rem;
	}
	.item {
		position: relative;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.25rem;
		padding: 0.25rem;
		background: var(--paper);
		color: var(--ink);
		border: 2px solid var(--border-muted);
		cursor: pointer;
		touch-action: manipulation;
		-webkit-touch-callout: none;
		user-select: none;
		transition: border-color 0.1s ease;
	}
	.item:hover {
		background: var(--paper);
		border-color: var(--ink);
	}
	.item.source {
		border-color: var(--ink);
		filter: drop-shadow(0 0 6px var(--primary-color));
	}
	/* the pack mode (research/PACKING.md §2d): never colour only – lit = an ink frame + the cyan glow, dimmed = faded +
	   the reason in words */
	.item.lit {
		border-color: var(--ink);
		box-shadow: 0 0 0 3px var(--primary-color);
	}
	.item.source {
		border-style: dashed;
	}
	/* the picture fades, the reason stays readable */
	.item.dim > :global(:not(.why)) {
		opacity: 0.35;
	}
	.item.drop-target {
		outline: 3px dashed var(--primary-color);
		outline-offset: -3px;
	}
	.name {
		font-size: 0.625rem;
		line-height: 1.2;
		text-align: center;
		overflow-wrap: anywhere;
	}
	.count {
		position: absolute;
		left: 0.25rem;
		top: 0.25rem;
		padding: 0 0.25rem;
		font-size: 0.75rem;
		background: var(--paper);
		border: 1px solid black;
	}
	.count.fill {
		width: auto;
		padding: 0 0.35rem;
		border-radius: 1rem;
	}
	.count.counting {
		background: var(--primary-color);
		color: black;
		font-size: 1rem;
	}
</style>
