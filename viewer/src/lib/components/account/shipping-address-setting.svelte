<script lang="ts">
	import type { ShippingResult } from '@scutoons/api';
	import { fetchShipping, setShipping } from '$lib/api.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { formatDate } from '$lib/utils/date.ts';

	/**
	 * Account page "Stammdaten" → the shipping address (#66, agents/research/CAMPAIGN-GIFTS.md §5; owner 2026-10-20:
	 * "die muss vom reader im account nach getragen werden"): physical gifts of a newsletter wait for it. The gifts still
	 * waiting say so above the form; saving moves them on to "to send". Only the account itself sees the address.
	 */
	let data = $state<ShippingResult | null>(null);
	let form = $state({ name: '', address: '', country: '' });
	let busy = $state(false);
	let message = $state<string | null>(null);
	let failed = $state(false);

	function fill(next: ShippingResult) {
		data = next;
		form = { name: next.address?.name ?? '', address: next.address?.address ?? '', country: next.address?.country ?? next.waiting[0]?.countries[0] ?? '' };
	}

	$effect(() => {
		fetchShipping().then(fill).catch(() => {});
	});

	async function save(e: SubmitEvent) {
		e.preventDefault();
		busy = true;
		failed = false;
		message = null;
		try {
			const before = data?.waiting.length ?? 0;
			const next = await setShipping({ name: form.name.trim(), address: form.address.trim(), country: form.country.trim().toUpperCase() });
			fill(next);
			message = before > next.waiting.length ? m['account.shipping_saved_gift']() : m['account.shipping_saved']();
		} catch {
			failed = true;
		} finally {
			busy = false;
		}
	}
</script>

{#if data}
	<form class="shipping" onsubmit={save} data-testid="shipping-address">
		<h3>{m['account.shipping_title']()}</h3>
		{#each data.waiting as gift (gift.id)}
			<p class="waiting" role="status">
				{m['account.shipping_waiting']({ name: gift.item, until: gift.until ? formatDate(new Date(gift.until)) : '' })}
				{#if gift.countries.length}{m['account.shipping_countries']({ countries: gift.countries.join(', ') })}{/if}
			</p>
		{/each}
		{#if !data.waiting.length && !data.address}<p class="hint">{m['account.shipping_hint']()}</p>{/if}
		<label for="ship-name">{m['items.ship_name']()}</label>
		<input id="ship-name" autocomplete="name" bind:value={form.name} />
		<label for="ship-address">{m['items.ship_address']()}</label>
		<textarea id="ship-address" rows="3" autocomplete="street-address" bind:value={form.address}></textarea>
		<label for="ship-country">{m['account.shipping_country']()}</label>
		<input id="ship-country" class="country" maxlength="2" autocomplete="country" bind:value={form.country} />
		{#if failed}<p class="error" role="alert">{m['account.shipping_failed']()}</p>{/if}
		{#if message}<p role="status">{message}</p>{/if}
		<div class="actions"><button type="submit" disabled={busy}>{m['items.ship_send']()}</button></div>
	</form>
{/if}

<style>
	.shipping {
		display: grid;
		gap: 0.25rem;
		margin: 1rem 0;
	}
	h3 {
		margin: 0 0 0.25rem;
	}
	label {
		margin-top: 0.5rem;
	}
	.country {
		width: 4rem;
		text-transform: uppercase;
	}
	.waiting {
		border: 2px solid black;
		padding: 0.5rem;
		margin: 0;
	}
	.hint {
		margin: 0;
	}
	.error {
		border: 2px solid black;
		padding: 0.5rem;
	}
	.actions {
		display: flex;
		justify-content: flex-end;
		margin-top: 0.5rem;
	}
</style>
