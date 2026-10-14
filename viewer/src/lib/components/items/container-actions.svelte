<script lang="ts">
	import type { Entry } from '@scutoons/content';
	import { moveWithWarning } from './container-move.ts';
	import { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * Containers in the item detail (agents/research/SCORES.md §2, owner 2026-10-14: moved by hand): a container opens
	 * its own grid; an item that fits a container of the reader's goes "into …" (one button per container); an item in a
	 * container comes out again. Past a soft limit (the DIY song book) the reader is warned first (in-page dialog).
	 */
	let { entry, onopen }: { entry: Entry; onopen: (container: Entry) => void } = $props();

	const inventory = getInventoryContext();
	const def = $derived(inventory.item(entry.item_id));
	const holds = $derived(def?.holds ?? null);
	const count = $derived(holds ? inventory.contentsOf(entry).reduce((n, e) => n + e.qty, 0) : 0);
	const targets = $derived(inventory.containersFor(entry));
	const inside = $derived(inventory.containerOf(entry));
	let working = $state(false);
	let error = $state<string | null>(null);

	async function into(container: Entry | null) {
		working = true;
		error = await moveWithWarning(inventory, entry, container);
		working = false;
	}
</script>

{#if holds}
	<button class="btn-secondary" onclick={() => onopen(entry)} data-testid="container-open">
		{m['items.container_open']()} · {count}{holds.max != null ? ` / ${holds.max}` : ''}
	</button>
	{#if entry.overfilled_at}<p class="note" role="status" data-testid="container-overfilled">{m['items.container_overfilled']()}</p>{/if}
{/if}
{#if inside}
	<button class="btn-secondary" disabled={working} onclick={() => into(null)} data-testid="container-take-out">{m['items.container_take_out']({ name: inventory.nameOf(inside) })}</button>
{/if}
{#each targets as c (c.id)}
	<button class="btn-secondary" disabled={working} onclick={() => into(c)} data-testid="container-put-in">{m['items.container_put_in']({ name: inventory.nameOf(c) })}</button>
{/each}
{#if error}<p class="note" role="alert">{error}</p>{/if}

<style>
	.note {
		width: 100%;
		margin: 0;
	}
</style>
