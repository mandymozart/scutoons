<script lang="ts">
	import { Sheet } from '@scutoons/ui';
	import type { GiftInfo } from '@scutoons/api';
	import { ApiError, claimGift, giftInfo } from '$lib/api.ts';
	import { giftLink } from '$lib/gift-link.svelte.ts';
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import { getLocale, localizeHref } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { formatDate } from '$lib/utils/date.ts';
	import ItemIcon from './item-icon.svelte';

	/**
	 * "Ein Geschenk wartet auf dich" (#66, agents/research/CAMPAIGN-GIFTS.md §5): the gift of a newsletter – the item,
	 * until when, and "Abholen". Virtual → into the inventory; physical → "comes by post" and the way to the account's
	 * shipping address. States: already yours, not yet, over, all given away, another account, signed out.
	 */
	const session = getSessionContext();
	const inventory = getInventoryContext();

	let info = $state<GiftInfo | null>(null);
	let error = $state<string | null>(null);
	let busy = $state(false);
	let open = $state(false);

	$effect(() => {
		const token = giftLink.token;
		if (!token) return;
		open = true;
		info = null;
		error = null;
		giftInfo(token, getLocale())
			.then((r) => (info = r.gift))
			.catch(() => (error = m['gift.invalid']()));
	});

	async function claim() {
		const token = giftLink.token;
		if (!token) return;
		busy = true;
		error = null;
		try {
			info = (await claimGift(token, getLocale())).gift;
			if (info.claim?.status === 'claimed') await session.loadMe(); // the new piece in the inventory
		} catch (e) {
			if (e instanceof ApiError && e.code === 'max_reached') error = m['gift.max_reached']();
			else if (e instanceof ApiError && (e.code === 'ended' || e.code === 'sold_out' || e.code === 'already_claimed')) info = info && { ...info, state: e.code === 'already_claimed' ? 'claimed' : e.code };
			else error = m['gift.failed']();
		} finally {
			busy = false;
		}
	}

	function close() {
		open = false;
		giftLink.close();
	}

	const until = $derived(info?.until ? formatDate(new Date(info.until)) : '');
	const signInHref = $derived(localizeHref('/account') + '?return=' + encodeURIComponent(`/?gift=${giftLink.token ?? ''}`));
</script>

<Sheet bind:open title={m['gift.title']()} closeLabel={m['common.close']()} onclose={close}>
	<div class="gift" data-testid="gift-sheet">
		{#if error && !info}
			<p>{error}</p>
		{:else if !info}
			<p class="muted">{m['common.loading']()}</p>
		{:else}
			<div class="item">
				<ItemIcon srcs={[inventory.iconOf({ item_id: info.item.id, variant: info.item.variant })]} name={info.item.name} size="6rem" />
				<p class="name">{info.item.name}</p>
			</div>

			{#if info.state === 'claimed'}
				{#if info.claim?.status === 'address'}
					<p>{m['gift.physical_address']()}</p>
					<a class="btn" href={localizeHref('/account/details')} onclick={close}>{m['gift.to_address']()}</a>
				{:else if info.claim?.status === 'ship' || info.claim?.status === 'shipped'}
					<p>{m['gift.physical_on_its_way']()}</p>
				{:else}
					<p>{m['gift.yours']()}</p>
				{/if}
			{:else if info.state === 'other_account'}
				<p>{m['gift.other_account']()}</p>
			{:else if info.state === 'ended'}
				<p>{m['gift.ended']()}</p>
			{:else if info.state === 'sold_out'}
				<p>{m['gift.sold_out']()}</p>
			{:else if info.state === 'not_started'}
				<p>{m['gift.not_started']({ from: info.starts_at ? formatDate(new Date(info.starts_at)) : '' })}</p>
			{:else}
				<p>{info.item.physical ? m['gift.open_physical']({ until }) : m['gift.open']({ until })}</p>
				{#if error}<p class="error" role="alert">{error}</p>{/if}
				{#if session.isLoggedIn}
					<button type="button" disabled={busy} onclick={claim} data-testid="gift-claim">{m['gift.claim']()}</button>
				{:else}
					<a class="btn" href={signInHref} onclick={close}>{m['gift.sign_in']()}</a>
				{/if}
			{/if}
		{/if}
	</div>
</Sheet>

<style>
	.gift {
		display: grid;
		gap: 1rem;
		justify-items: center;
		text-align: center;
		padding: 1rem;
	}
	.item {
		display: grid;
		justify-items: center;
		gap: 0.5rem;
	}
	.name {
		margin: 0;
		font-size: 1.25rem;
	}
	.error {
		border: 2px solid black;
		padding: 0.5rem;
	}
</style>
