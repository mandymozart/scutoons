<script lang="ts">
	import { getMembershipContext } from '$lib/contexts/membership.svelte.ts';
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import { isMembershipExpired } from '$lib/auth/index.ts';
	import { localizeHref } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { formatDate } from '$lib/utils/date.ts';
	import accountIcon from '$lib/icons/account.svg';
	import LoginForm from '$lib/components/account/login-form.svelte';
	import Avatar from '$lib/components/avatar/avatar.svelte';

	const membershipCtx = getMembershipContext();
	const session = getSessionContext();
	const inventory = getInventoryContext();

	const itemCount = $derived(inventory.entries.reduce((sum, e) => sum + e.qty, 0));

	const typeLabel = (type: string) => (m as any)[`account.${type}`]?.() ?? type;

</script>

<div class="view view-account">
	<header class="account-header">
		<h1><img src={accountIcon} alt="Account" /> {m['account.my_account']()}</h1>
	</header>

	{#if !session.ready && session.token}
		<section><p>{m['common.loading']()}</p></section>
	{:else if session.isLoggedIn && session.user}
		<section>
			<table>
				<tbody>
					<tr><td>{m['account.signed_in_as']()}</td><td>{session.user.email}</td></tr>
					{#if session.user.created_at}
						<tr
							><td>{m['account.created_at']()}</td><td
								>{formatDate(new Date(session.user.created_at.replace(' ', 'T')))}</td
							></tr
						>
					{/if}
				</tbody>
			</table>
			<button class="btn btn-sign-out" onclick={() => session.logout()}>{m['account.sign_out']()}</button>
		</section>

		<section>
			<h2>{m['account.memberships']()}</h2>
			{#if session.memberships.length === 0}
				<p>{m['account.no_memberships']()}</p>
			{:else}
				<table>
					<tbody>
						{#each session.memberships as ms}
							<tr>
								<td>{typeLabel(ms.type)}</td>
								<td>
									{ms.code} · {ms.expires_at || m['account.lifetime']()}
									{#if isMembershipExpired(ms.expires_at)}(✕){/if}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			{/if}
		</section>
	{:else}
		<section>
			<h2>{m['account.login_title']()}</h2>
			{#if membershipCtx.pendingCode}
				<p class="notification">{m['claim.legacy_prompt']()} ({membershipCtx.pendingCode})</p>
			{/if}
			<!-- a legacy device membership code is claimed together with the login -->
			<LoginForm code={membershipCtx.pendingCode} />
		</section>
	{/if}

	<section>
		<h2>{m['account.inventory']()}</h2>
		<p class="inventory-line">
			<span>{m['account.items_count']({ count: itemCount })}</span>
			<a class="btn btn-secondary" href={localizeHref('/catpope') + '?inventory=open'}>
				{m['account.open_inventory']()}
			</a>
		</p>
	</section>

	<section>
		<h2>{m['account.avatar']()}</h2>
		<Avatar />
	</section>

</div>

<style>
	.view.view-account {
		max-width: 600px;
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
	h2 {
		font-size: 1rem;
		font-weight: 700;
		margin: 0 0 0.75rem;
	}
	table {
		width: 100%;
		border-collapse: collapse;
		margin-bottom: 1rem;
	}
	td {
		padding: 0.25rem 0.5rem;
		vertical-align: top;
		overflow-wrap: anywhere;
	}
	td:first-child {
		background: #f0f0f0;
		width: 40%;
	}
	.inventory-line {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		flex-wrap: wrap;
		margin: 0;
	}
	.inventory-line .btn {
		padding: 0.75rem 1rem;
		text-decoration: none;
	}
	.notification {
		margin: 0 0 1rem;
	}
</style>
