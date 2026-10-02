<script lang="ts">
	import { getInventoryContext, localized } from '$lib/contexts/inventory.svelte.ts';
	import { getItemsUiContext } from '$lib/contexts/items-ui.svelte.ts';
	import { isGiftable, type Entry } from '@scutoons/content';
	import { Field } from '@scutoons/ui';
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
	// a substance (ITEMS.md "Substances"): "Benutzen" uses one piece up and applies it
	const substance = $derived(skills.some((s) => s.type === 'apply'));
	// food (cucumber, tuna can) is eaten: "Essen" instead of "Benutzen", same mechanics (appetite down)
	const useLabel = $derived(def?.category === 'food' ? m['items.eat']() : m['items.use']());
	let useError = $state<string | null>(null);
	async function useIt() {
		useError = null;
		working = true;
		const result = await inventory.useSubstance(entry);
		working = false;
		if (!result.ok) useError = result.error ?? 'error';
	}
	const combinable = $derived(
		(inventory.catalog?.recipes ?? []).some((r) => r.inputs.some((i) => i.item === entry.item_id))
	);
	const equipped = $derived(inventory.isEquipped(entry));
	const total = $derived(inventory.editionTotal(entry.item_id));
	// "Anziehen" swaps (ITEMS.md "Wardrobe"): what was on that slot goes back into the inventory, or – while "Hide
	// sensitive content" is on – the swim pants come back on; the newest such note is shown under the buttons
	const note = $derived([...inventory.notices].reverse().find((n) => n.kind === 'returned' || n.kind === 'restored'));
	const noteText = $derived(
		note ? (note.kind === 'returned' ? m['items.returned'] : m['items.restored'])({ name: inventory.nameOf({ item_id: note.item_id, variant: null }) }) : ''
	);
	let working = $state(false);

	const sourceLabel = $derived(
		({
			qr: m['items.source_qr'](),
			story: m['items.source_story'](),
			scan: m['items.source_scan'](),
			craft: m['items.source_craft'](),
			starter: m['items.source_starter'](),
			gift: m['items.source_gift']()
		})[entry.source] ?? entry.source
	);

	// "Verschenken" (ITEMS.md "Ownership, claims, gifting"): members only, giftable items; a worn piece comes off first
	// (a stackable keeps its worn piece back); the server checks everything again (cooldown, daily limit, recipient)
	const giftable = $derived(inventory.isAccount && isGiftable(def));
	const giftMax = $derived(Math.max(0, entry.qty - (equipped ? 1 : 0)));
	let giving = $state(false);
	let giftTo = $state('');
	let giftQty = $state(1);
	let giftError = $state<string | null>(null);
	const GIFT_ERRORS = ['recipient_not_found', 'worn', 'cooldown', 'max_reached', 'daily_limit', 'self_gift', 'invalid_email', 'not_giftable'];
	async function sendGift(event: SubmitEvent) {
		event.preventDefault();
		if (!giftTo.includes('@')) {
			giftError = m['items.gift_error_invalid_email']();
			return;
		}
		giftError = null;
		working = true;
		const result = await inventory.gift(entry, giftTo, Math.min(Math.max(1, giftQty), giftMax || 1));
		working = false;
		if (result.ok) {
			giving = false;
			onback();
			return;
		}
		const code = GIFT_ERRORS.includes(result.error ?? '') ? result.error : 'failed';
		giftError = (m as unknown as Record<string, () => string>)[`items.gift_error_${code}`]();
	}

	async function toggleWear() {
		if (!def?.slot) return;
		working = true;
		try {
			if (equipped) await inventory.unequipEntry(entry);
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
		{#if equipped}
			<dt>{m['items.tab_avatar']()}</dt>
			<dd data-testid="item-worn">{m['items.worn']()}</dd>
		{/if}
	</dl>

	<div class="actions">
		{#if usable}
			<button class="btn-secondary" class:disabled={working} disabled={working} onclick={() => (substance ? useIt() : ui.use(entry, skills))} data-testid="item-use">{useLabel}</button>
		{/if}
		{#if combinable}
			<button class="btn-secondary" onclick={() => oncombine(entry)}>{m['items.combine']()}</button>
		{/if}
		{#if def?.slot}
			<button class="btn-secondary" class:disabled={working} disabled={working} onclick={toggleWear}>
				{equipped ? m['items.take_off']() : m['items.put_on']()}
			</button>
		{/if}
		{#if giftable && !giving}
			<button class="btn-secondary" onclick={() => ((giving = true), (giftError = null), (giftQty = 1))} data-testid="item-gift">{m['items.gift']()}</button>
		{/if}
	</div>
	{#if giving}
		<form class="gift" onsubmit={sendGift} data-testid="gift-form">
			<h3>{m['items.gift_title']({ name: inventory.nameOf(entry) })}</h3>
			{#if giftMax < 1}
				<p class="swap-note" role="status">{m['items.gift_worn']()}</p>
			{:else}
				<Field label={m['items.gift_to']()} error={giftError}>
					{#snippet children({ id, describedby, invalid })}
						<input {id} type="email" name="gift_to" autocomplete="off" inputmode="email" required aria-describedby={describedby} aria-invalid={invalid} bind:value={giftTo} />
					{/snippet}
				</Field>
				{#if giftMax > 1}
					<Field label={m['items.gift_qty']()}>
						{#snippet children({ id, describedby })}
							<input {id} type="number" name="gift_qty" min="1" max={giftMax} aria-describedby={describedby} bind:value={giftQty} />
						{/snippet}
					</Field>
				{/if}
				<p class="gift-note">{m['items.gift_note']()}{entry.code ? ` ${m['items.gift_note_code']()}` : ''}</p>
				<div class="actions">
					<button type="button" class="btn-secondary" onclick={() => (giving = false)}>{m['items.gift_cancel']()}</button>
					<button type="submit" class:disabled={working} disabled={working} data-testid="gift-confirm">{m['items.gift_confirm']()}</button>
				</div>
			{/if}
		</form>
	{/if}
	{#if noteText}<p class="swap-note" role="status" data-testid="item-note">{noteText}</p>{/if}
	{#if useError}<p class="swap-note" role="alert">{m['items.use_failed']()}</p>{/if}
</div>

<style>
	.item-detail {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.btn-back {
		align-self: flex-start;
		color: var(--ink);
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
		color: var(--text-muted);
	}
	dd {
		margin: 0;
	}
	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}
	.swap-note {
		margin: 0;
		font-size: 0.75rem;
		line-height: 1.3;
	}
	.actions button {
		flex: 1 1 auto;
		padding: 0.75rem 1rem;
	}
	.gift {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		border-top: 2px solid var(--ink, black);
		padding-top: 0.75rem;
	}
	.gift h3 {
		margin: 0;
		font-size: 1rem;
	}
	.gift input {
		width: 100%;
		box-sizing: border-box;
	}
	.gift-note {
		margin: 0;
		font-size: 0.75rem;
		line-height: 1.3;
	}
</style>
