<script lang="ts">
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { readerName } from '$lib/reader-name.svelte.ts';
	import { getItemsUiContext } from '$lib/contexts/items-ui.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The account index's top for a member (owner 2026-10-16: "Name kann prominenter"): the reader's name large (the
	 * address when there is none), the address under it, and what the account unlocks; without access, the way to it.
	 */
	const session = getSessionContext();
	const itemsUi = getItemsUiContext();
	/** access only through the promo code the account was created with (not owned) */
	const promoAccessCode = $derived.by(() => {
		const access = session.me?.access;
		return access?.signup_code && access.codes.length === 1 ? access.signup_code : null;
	});
	const name = $derived(readerName.value);
</script>

<section class="card" aria-label={m['account.group_details']()} data-testid="account-card">
	<p class="who">
		<strong class="name">{name ?? session.user?.email}</strong>
		{#if name}<small>{session.user?.email}</small>{/if}
	</p>
	<p class="access">
		<small>{m['account.access']()}</small>
		<span>
			{#if session.hasAccess && promoAccessCode}
				{m['account.access_promo']({ code: promoAccessCode })}
			{:else if session.hasAccess}
				{m['account.access_unlocked']({ until: session.me?.access?.expires_at || m['account.lifetime']() })}
			{:else}
				{m['account.access_none']()}
			{/if}
		</span>
	</p>
	{#if !session.hasAccess}
		<button type="button" class="btn btn-secondary" onclick={() => itemsUi.openCamera({ qr: true, object: null, entry: null })}>{m['membership_blocker.scan']()}</button>
	{/if}
</section>

<style>
	.card {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		border: 2px solid var(--ink);
		padding: 1rem;
	}
	p {
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
		overflow-wrap: anywhere;
	}
	.name {
		font-size: 1.5rem;
		line-height: 1.2;
	}
	small {
		font-size: 0.75rem;
		line-height: 1.3;
		color: var(--text-muted);
	}
	.btn {
		align-self: flex-start;
		padding: 0 1rem;
		text-decoration: none;
	}
</style>
