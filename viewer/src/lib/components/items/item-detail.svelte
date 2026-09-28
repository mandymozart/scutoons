<script lang="ts">
	import { getInventoryContext, localized } from '$lib/contexts/inventory.svelte.ts';
	import { getItemsUiContext } from '$lib/contexts/items-ui.svelte.ts';
	import type { Entry } from '$lib/models/items.ts';
	import { m } from '$lib/paraglide/messages.js';
	import ItemIcon from './item-icon.svelte';

	let {
		entry,
		onback,
		oncombine
	}: { entry: Entry; onback: () => void; oncombine: (entry: Entry) => void } = $props();

	const inventory = getInventoryContext();
	const ui = getItemsUiContext();

	const def = $derived(inventory.item(entry.item_id));
	const skills = $derived(def?.skills ?? []);
	const usable = $derived(skills.length > 0);
	const combinable = $derived(
		(inventory.catalog?.recipes ?? []).some((r) => r.inputs.some((i) => i.item === entry.item_id))
	);
	const equipped = $derived(inventory.isEquipped(entry));
	const total = $derived(inventory.editionTotal(entry.item_id));
	let working = $state(false);

	const sourceLabel = $derived(
		({
			qr: m['items.source_qr'](),
			story: m['items.source_story'](),
			scan: m['items.source_scan'](),
			craft: m['items.source_craft'](),
			starter: m['items.source_starter']()
		})[entry.source] ?? entry.source
	);

	async function toggleWear() {
		if (!def?.slot) return;
		working = true;
		try {
			if (equipped) await inventory.unequip(def.slot);
			else await inventory.equip(entry);
		} catch (e) {
			console.warn('Avatar update failed', e);
		} finally {
			working = false;
		}
	}
</script>

<div class="item-detail">
	<button class="btn-back btn-clear" onclick={onback}>← {m['items.back']()}</button>

	<div class="hero">
		<ItemIcon srcs={[inventory.iconOf(entry), def?.icon ?? '']} name={inventory.nameOf(entry)} />
	</div>

	<h2>{inventory.nameOf(entry)}</h2>
	{#if def}
		<p class="description">{localized(def.description)}</p>
	{/if}

	<dl class="meta">
		{#if entry.edition_number}
			<dt>Edition</dt>
			<dd>
				{total
					? m['items.edition']({ number: entry.edition_number, total })
					: m['items.edition_number']({ number: entry.edition_number })}
			</dd>
		{/if}
		{#if entry.qty > 1}
			<dt>{m['items.qty']()}</dt>
			<dd>{entry.qty}</dd>
		{/if}
		<dt>{m['items.source']()}</dt>
		<dd>{sourceLabel}{entry.code ? ` · ${entry.code}` : ''}</dd>
	</dl>

	<div class="actions">
		{#if usable}
			<button onclick={() => ui.use(entry, skills)}>{m['items.use']()}</button>
		{/if}
		{#if combinable}
			<button class="btn-secondary" onclick={() => oncombine(entry)}>{m['items.combine']()}</button>
		{/if}
		{#if def?.slot}
			<button class="btn-secondary" class:disabled={working} disabled={working} onclick={toggleWear}>
				{equipped ? m['items.take_off']() : m['items.put_on']()}
			</button>
		{/if}
	</div>
</div>

<style>
	.item-detail {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.btn-back {
		align-self: flex-start;
		color: black;
		font-size: 0.75rem;
	}
	.hero {
		width: 10rem;
		max-width: 60%;
		align-self: center;
	}
	h2 {
		margin: 0;
		font-size: 1rem;
		font-weight: 700;
	}
	.description {
		margin: 0;
	}
	.meta {
		display: grid;
		grid-template-columns: auto 1fr;
		gap: 0.25rem 1rem;
		margin: 0.5rem 0;
		font-size: 0.75rem;
		line-height: 1.3;
	}
	dt {
		color: #6b6b6b;
	}
	dd {
		margin: 0;
	}
	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}
	.actions button {
		flex: 1 1 auto;
		padding: 0.75rem 1rem;
	}
</style>
