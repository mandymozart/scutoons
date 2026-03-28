<script lang="ts">
	import { getMembershipContext } from '$lib/contexts/membership.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { formatDate } from '$lib/utils/date.ts';

	const membershipCtx = getMembershipContext();
	const membership = $derived(membershipCtx.membership);

	const expiryText = $derived(membership?.expires_at || 'Lifetime');
	const claimedText = $derived(membership?.claimed_at || 'Not claimed');
	const isClaimed = $derived(membership?.email || false);
	function signOut() {
		membershipCtx.clear();
	}
</script>

{#if membership}
	<div class="view account">
		<header class="account-header">
			<h1><img src="/icons/account.svg" alt="Account" /> {m['account.my_account']()}</h1>
		</header>

		{#if !isClaimed}
			<div class="notification">
				<p>
					🔔 {m['account.claim_membership_reminder']()} 
				</p>
				<p>
					<a
						href="https://qr.scutoons.com/?c={membership.code}"
						target="_blank"
						class="btn"
						rel="noopener noreferrer"
					>
						{m['account.claim_membership']()}
					</a>
				</p>
			</div>
		{/if}

		<section>
			<table>
				<tbody>
					<tr><td>{m['account.type']()}</td><td>{m[`account.${membership.type}`]()}</td></tr>
					<tr><td>{m['account.status']()}</td><td>{isClaimed ? m['account.claimed']() : m['account.not_claimed']}</td></tr>
					<tr><td>{m['account.valid_until']()}</td><td>{expiryText}</td></tr>
					<tr><td>{m['account.code']()}</td><td>{membership.code}</td></tr>
				</tbody>
			</table>
			<button class="btn btn-sign-out" onclick={signOut}>{m['account.sign_out']()}</button>
		</section>

		
		{#if membership.email}
			<section>
				<h2>{m['account.account_data']()}</h2>
				<table>
					<tbody>
						<tr><td>{m['account.email']()}</td><td>{membership.email}</td></tr>
					</tbody>
				</table>
			</section>
		{/if}

		<section>
			<h2>{m['account.timeline']()}</h2>
			<table>
				<tbody>
					<tr><td>{m['account.created_at']()}</td><td>{formatDate(new Date(membership.created_at))}</td></tr>
					<tr><td>{m['account.claimed_at']()}</td><td>{membership.claimed_at ? formatDate(new Date(membership.claimed_at)) : m['account.not_claimed']()}</td></tr>
					<tr><td>{m['account.validated_at']()}</td><td>{formatDate(new Date(membership.validated_at))}</td></tr>
				</tbody>
			</table>
		</section>

		<section>
			<h2>{m['account.technical']()}</h2>
			<div class="technical">
				{m['account.redirect_to']()}:<br/>
				{membership.redirect_to}
			</div>
		</section>
	</div>
{:else}
	<div class="view account-view">
		<h1>{m['account.my_account']()}</h1>
		<p>{m['account.no_membership_message']()}</p>
	</div>
{/if}

<style>
	.view.account {
		max-width: 600px;
		margin: 5rem auto;
	}
	header{
		padding: 0 1rem;
		h1 {
			display: flex;
			align-items: center;
			gap: .5rem;
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
	.technical {
		background-color: #f5f5f5;
		padding: 1rem;
	}
</style>