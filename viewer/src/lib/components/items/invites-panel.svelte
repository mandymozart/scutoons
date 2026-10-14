<script lang="ts">
	import { Field } from '@scutoons/ui';
	import { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * Gift invites in the inventory (agents/research/GIFT-INVITES.md), members only:
	 * - a campaign that gives its own item: "Invite someone" – an address, the campaign's item goes with the invite
	 *   (a physical one the member pays: the price, then the gift waits for the payment)
	 * - "On its way": the member's open invites (waiting / payment open) with "Call back" while not accepted
	 * - a physical gift the member joined by: where to send it (name, address, country)
	 * Nothing shows when there is nothing to show.
	 */
	const inventory = getInventoryContext();
	$effect(() => {
		if (inventory.isAccount) void inventory.loadInvites();
	});

	const data = $derived(inventory.invites);
	const offer = $derived(data?.campaign?.gift === 'campaign' && data.campaign.left > 0 ? data.campaign : null);
	const open = $derived((data?.invites ?? []).filter((i) => i.status === 'waiting' || i.status === 'payment'));
	const date = (iso: string | null) => (iso ? new Date(iso).toLocaleDateString(getLocale()) : '');
	const money = (cents: number | null, currency: string | null) =>
		cents === null ? '' : new Intl.NumberFormat(getLocale(), { style: 'currency', currency: currency ?? 'EUR' }).format(cents / 100);

	// ── invite with the campaign's item ──
	let to = $state('');
	let inviteError = $state<string | null>(null);
	let working = $state(false);
	const INVITE_ERRORS = ['has_account', 'already_invited', 'campaign_limit', 'sold_out', 'no_campaign', 'daily_limit', 'self_gift', 'invalid_email'];
	async function sendInvite(event: SubmitEvent) {
		event.preventDefault();
		if (!to.includes('@')) {
			inviteError = m['items.gift_error_invalid_email']();
			return;
		}
		working = true;
		inviteError = null;
		const result = await inventory.invite(to);
		working = false;
		if (result.ok) {
			to = '';
			return;
		}
		const code = INVITE_ERRORS.includes(result.error ?? '') ? result.error : 'failed';
		inviteError = (m as unknown as Record<string, () => string>)[code === 'self_gift' || code === 'invalid_email' ? `items.gift_error_${code}` : `items.invite_error_${code}`]();
	}

	// ── call back ──
	let recallError = $state<string | null>(null);
	async function recall(id: string) {
		recallError = null;
		const result = await inventory.recallInvite(id);
		if (!result.ok) recallError = result.error === 'not_open' ? m['items.invite_error_not_open']() : m['items.invite_error_failed']();
	}

	// ── the shipping address of a physical gift ──
	/** an address was saved: one line says so (the form itself goes) */
	let shipSaved = $state(false);
	let ship = $state<Record<string, { name: string; address: string; country: string; error: string | null }>>({});
	// one form per gift that waits for an address (made here, not while rendering)
	$effect(() => {
		for (const gift of data?.address_needed ?? []) {
			if (!ship[gift.id]) ship[gift.id] = { name: '', address: '', country: gift.countries[0] ?? '', error: null };
		}
	});
	async function sendAddress(id: string) {
		const form = ship[id];
		if (!form) return;
		if (!form.name.trim() || !form.address.trim()) {
			form.error = m['items.ship_error_missing']();
			return;
		}
		const result = await inventory.setInviteAddress(id, { name: form.name.trim(), address: form.address.trim(), country: form.country });
		form.error = result.ok ? null : result.error === 'invalid_country' ? m['items.ship_error_country']() : m['items.invite_error_failed']();
		if (result.ok) shipSaved = true;
	}
</script>

{#if data && (offer || open.length || data.address_needed.length || shipSaved)}
	<section class="invites" data-testid="invites">
		{#if shipSaved && !data.address_needed.length}<p class="card note" role="status">{m['items.ship_saved']()}</p>{/if}
		{#each data.address_needed as gift (gift.id)}
			{#if ship[gift.id]}
			{@const form = ship[gift.id]}
			<form class="card" onsubmit={(e) => (e.preventDefault(), sendAddress(gift.id))} data-testid="ship-form">
				<h3>{m['items.ship_title']({ name: gift.item })}</h3>
				<p class="note">{m['items.ship_hint']()}</p>
				<Field label={m['items.ship_name']()}>
					{#snippet children({ id })}<input {id} autocomplete="name" bind:value={form.name} />{/snippet}
				</Field>
				<Field label={m['items.ship_address']()}>
					{#snippet children({ id })}<textarea {id} rows="3" autocomplete="street-address" bind:value={form.address}></textarea>{/snippet}
				</Field>
				<Field label={m['items.ship_country']()} error={form.error}>
					{#snippet children({ id })}
						<select {id} bind:value={form.country}>
							{#each gift.countries as c (c)}<option value={c}>{c}</option>{/each}
						</select>
					{/snippet}
				</Field>
				<div class="actions"><button type="submit">{m['items.ship_send']()}</button></div>
			</form>
			{/if}
		{/each}

		{#if offer}
			<form class="card" onsubmit={sendInvite} data-testid="invite-form">
				<h3>{m['items.invite_title']()}</h3>
				<p class="note">
					{m['items.invite_campaign_hint']({ name: inventory.nameOf({ item_id: offer.item_id ?? '', variant: offer.variant }), until: date(offer.wait_until) })}
					{#if offer.price_cents !== null}{m['items.invite_price']({ price: money(offer.price_cents, offer.currency) })}{/if}
				</p>
				<Field label={m['items.gift_to']()} error={inviteError}>
					{#snippet children({ id, describedby, invalid })}
						<input {id} type="email" autocomplete="off" inputmode="email" required aria-describedby={describedby} aria-invalid={invalid} bind:value={to} />
					{/snippet}
				</Field>
				<p class="note">{offer.price_cents !== null ? m['items.invite_note_paid']() : m['items.invite_note']()}</p>
				<div class="actions">
					<button type="submit" class:disabled={working} disabled={working}>{offer.price_cents !== null ? m['items.invite_confirm_paid']() : m['items.invite_send']()}</button>
				</div>
			</form>
		{/if}

		{#if open.length}
			<div class="card">
				<h3>{m['items.invites_open']()}</h3>
				{#if recallError}<p class="note" role="alert">{recallError}</p>{/if}
				<ul>
					{#each open as invite (invite.id)}
						<li data-testid="invite-{invite.id}">
							<span class="what">{invite.qty > 1 ? `${invite.qty} × ` : ''}{invite.item}</span>
							<span class="note">{m['items.invite_to']({ email: invite.email })}</span>
							<span class="state" class:payment={invite.status === 'payment'}>
								{invite.status === 'payment'
									? m['items.invite_payment']({ price: money(invite.price_cents, invite.currency), until: date(invite.payment_due_at) })
									: m['items.invite_waiting']({ until: date(invite.expires_at) })}
							</span>
							<button type="button" class="btn-secondary" onclick={() => recall(invite.id)}>
								{invite.status === 'payment' ? m['items.invite_cancel']() : m['items.invite_recall']()}
							</button>
						</li>
					{/each}
				</ul>
			</div>
		{/if}
	</section>
{/if}

<style>
	.invites {
		display: grid;
		gap: 0.75rem;
		margin-bottom: 1rem;
	}
	.card {
		display: grid;
		gap: 0.5rem;
		padding: 0.75rem;
		border: var(--border-width-regular) solid var(--ink);
	}
	h3 {
		margin: 0;
		font-size: 1rem;
	}
	.note {
		margin: 0;
		font-size: 0.75rem;
		line-height: 1.3;
	}
	ul {
		display: grid;
		gap: 0.75rem;
		margin: 0;
		padding: 0;
		list-style: none;
	}
	li {
		display: grid;
		gap: 0.25rem;
		justify-items: start;
	}
	.state {
		font-size: 0.75rem;
		padding: 0 0.3rem;
		border: var(--border-width-hairline) solid var(--ink);
	}
	.state.payment {
		border-color: var(--state-warning);
		color: var(--state-warning);
	}
	.actions {
		display: flex;
		justify-content: flex-end;
	}
	textarea {
		width: 100%;
	}
</style>
