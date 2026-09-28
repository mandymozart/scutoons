<script lang="ts">
	import { goto } from '$app/navigation';
	import { localizeHref } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { getMembershipContext } from '$lib/contexts/membership.svelte.ts';
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { getItemsUiContext } from '$lib/contexts/items-ui.svelte.ts';
	import type { Snippet } from 'svelte';
	import accountIcon from '$lib/icons/account.svg';

	let {
		isPreview = false,
		children
	}: {
		isPreview?: boolean;
		children: Snippet;
	} = $props();

	const membership = getMembershipContext();
	const session = getSessionContext();
	const itemsUi = getItemsUiContext();

	const shouldShowContent = $derived(isPreview || membership.isValid);
	// don't flash the blocker while a stored session is still loading /me/
	const loading = $derived(!!session.token && !session.ready);

	const handleSignIn = () => {
		if (membership.pendingCode) itemsUi.openClaim(membership.pendingCode);
		else goto(localizeHref('/account'));
	};

	const handleBuyAccess = () => {
		goto(localizeHref('/shop'));
	};
</script>

{#if shouldShowContent}
	{@render children()}
{:else if loading}
	<p class="loading">{m['common.loading']()}</p>
{:else}
	<div class="view view-membership-blocker">
		<header>
			<h1><img src={accountIcon} alt="Account" />  {m['membership_blocker.title']()}</h1>
		</header>
		<section class="membership-required">
			<div class="membership-card">
				<p>{m['membership_blocker.description']()}</p>

				<div class="membership-info">
					<h3>{m['membership_blocker.how_to_access']()}</h3>
					<ol>
						<li>{m['membership_blocker.step_1']()}</li>
						<li>{m['membership_blocker.step_2']()}</li>
						<li>{m['membership_blocker.step_3']()}</li>
					</ol>
				</div>

				{#if session.isLoggedIn}
					<p>{m['membership_blocker.signed_in_hint']()}</p>
				{/if}

				<div class="membership-actions">
					{#if session.isLoggedIn}
						<button onclick={() => itemsUi.openCamera({ qr: true, object: null, entry: null })}>
							{m['camera.scan_code']()}
						</button>
					{:else}
						<button onclick={handleSignIn}>{m['membership_blocker.sign_in']()}</button>
					{/if}
					<button onclick={handleBuyAccess} class="btn-buy-access btn-secondary">
						{m['account.buy_access']()}
					</button>
				</div>
			</div>
		</section>
	</div>
{/if}

<style>
	.view.view-membership-blocker {
		max-width: 800px;
		margin: 5rem auto;
	}
	header {
		padding: 0 1rem;
		h1 {
			display: flex;
			align-items: center;
			gap: 0.5rem;
		}
		img {
			width: 4rem;
		}
		display: grid;
		grid-template-columns: auto 1fr;
		align-items: center;
		gap: 1rem;
	}
	section {
		padding: 1rem;
	}
	.membership-actions {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
		margin-top: 1rem;
	}
</style>
