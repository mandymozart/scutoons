<script lang="ts">
	import type { Entry } from '@scutoons/content';
	import { Icon, IconButton } from '@scutoons/ui';
	import { moveWithWarning } from './container-move.ts';
	import { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * Containers in the item detail (research/SCORES.md §2, research/PACKING.md §2a – owner 2026-10-16): a container
	 * opens its own grid; an item that some container of the reader's takes gets ONE small round icon, "Pack", which
	 * switches the inventory into the pack mode (the reader taps the target there); an item in a container comes out with
	 * a round icon – except from furniture, which is emptied at home.
	 */
	let { entry, onopen, onpack }: { entry: Entry; onopen: (container: Entry) => void; onpack: (entry: Entry) => void } = $props();

	const inventory = getInventoryContext();
	const def = $derived(inventory.item(entry.item_id));
	const holds = $derived(def?.holds ?? null);
	const count = $derived(holds ? inventory.countIn(entry) : 0);
	const inside = $derived(inventory.containerOf(entry));
	const atHomeOnly = $derived(!!inside && !!inventory.item(inside.item_id)?.furniture);
	const canPack = $derived(!inventory.isEquipped(entry) && inventory.packable(entry));
	let working = $state(false);
	let error = $state<string | null>(null);

	async function takeOut() {
		working = true;
		error = (await moveWithWarning(inventory, entry, null)).error;
		working = false;
	}
</script>

{#if holds}
	<button class="btn-secondary" onclick={() => onopen(entry)} data-testid="container-open">
		{m['items.container_open']()} · {count}{holds.max != null ? ` / ${holds.max}` : ''}
	</button>
	{#if entry.overfilled_at}<p class="note" role="status" data-testid="container-overfilled">{m['items.container_overfilled']()}</p>{/if}
{/if}
{#if inside && !atHomeOnly}
	<IconButton label={m['items.container_take_out']({ name: inventory.nameOf(inside) })} disabled={working} onclick={takeOut} data-testid="container-take-out"><Icon name="corner-up-left" size="1rem" /></IconButton>
{/if}
{#if canPack}
	<IconButton label={m['items.pack']()} onclick={() => onpack(entry)} data-testid="item-pack"><Icon name="package" size="1rem" /></IconButton>
{/if}
{#if error}<p class="note" role="alert">{error}</p>{/if}

<style>
	.note {
		width: 100%;
		margin: 0;
	}
</style>
