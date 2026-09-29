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
	/** access only through the promo code the account was created with (not owned) */
	const promoAccessCode = $derived.by(() => {
		const access = session.me?.access;
		return access?.signup_code && access.codes.length === 1 ? access.signup_code : null;
	});
	const roleLabel = (role: string) => (m as any)[`account.role_${role}`]?.() ?? role;

	// "Send me news about new releases" (PUT /me/)
	let optInSaving = $state(false);
	let optInError = $state<string | null>(null);
	async function toggleOptIn(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const value = input.checked;
		optInSaving = true;
		optInError = null;
		try {
			await session.setMarketingOptIn(value);
		} catch {
			input.checked = !value;
			optInError = m['account.marketing_opt_in_failed']();
		} finally {
			optInSaving = false;
		}
	}

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
					<tr>
						<td>{m['account.signed_in_as']()}</td>
						<td>{session.user.email}</td>
					</tr>
					{#if session.roles.length}
						<tr>
							<td>{m['account.roles']()}</td>
							<td>
								<span class="roles">
									{#each session.roles as role}
										<span class="role-tag">{roleLabel(role)}</span>
									{/each}
								</span>
							</td>
						</tr>
					{/if}
					<tr>
						<td>{m['account.access']()}</td>
						<td>
							{#if session.hasAccess && promoAccessCode}
								{m['account.access_promo']({ code: promoAccessCode })}
							{:else if session.hasAccess}
								{m['account.access_unlocked']({
									until: session.me?.access?.expires_at || m['account.lifetime']()
								})}
							{:else}
								{m['account.access_none']()}
							{/if}
						</td>
					</tr>
					{#if session.user.created_at}
						<tr
							><td>{m['account.created_at']()}</td><td
								>{formatDate(new Date(session.user.created_at.replace(' ', 'T')))}</td
							></tr
						>
					{/if}
				</tbody>
			</table>
			<label class="opt-in">
				<input
					type="checkbox"
					checked={!!session.user.marketing_opt_in}
					disabled={optInSaving}
					onchange={toggleOptIn}
				/>
				<span>
					{m['account.marketing_opt_in']()}
					<small>{m['account.marketing_opt_in_hint']()}</small>
				</span>
			</label>
			{#if optInError}
				<p class="error" role="alert">{optInError}</p>
			{/if}
			{#if session.hasRole('creator')}
				<p class="creator-line">
					<a class="btn btn-secondary" href={localizeHref('/creator')}>{m['account.creator_link']()}</a>
					<small>{m['account.creator_hint']()}</small>
				</p>
			{/if}
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
			<p>{m['account.login_hint']()}</p>
			<LoginForm />
			<p class="signup-rule">{m['account.signup_rule']()}</p>
		</section>
	{/if}

	{#if membershipCtx.deviceValid && membershipCtx.device}
		<section>
			<h2>{m['account.device_access']()}</h2>
			<p>
				{m['account.device_access_info']({
					code: membershipCtx.device.code,
					until: membershipCtx.device.expires_at || m['account.lifetime']()
				})}
			</p>
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
	.roles {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem;
	}
	.role-tag {
		display: inline-block;
		border: 2px solid black;
		padding: 0 0.375rem;
		font-size: 0.75rem;
		line-height: 1.4;
	}
	.opt-in {
		display: flex;
		align-items: flex-start;
		gap: 0.5rem;
		margin: 0 0 1rem;
		cursor: pointer;
	}
	.opt-in input {
		margin: 0.125rem 0 0;
		width: 1.25rem;
		height: 1.25rem;
		flex: none;
		accent-color: black;
	}
	.opt-in span {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}
	.opt-in small,
	.creator-line small,
	.signup-rule {
		font-size: 0.75rem;
		line-height: 1.3;
		color: #6b6b6b;
	}
	.signup-rule {
		margin: 1rem 0 0;
	}
	.creator-line {
		display: flex;
		align-items: center;
		gap: 1rem;
		flex-wrap: wrap;
		margin: 0 0 1rem;
	}
	.creator-line .btn {
		padding: 0.75rem 1rem;
		text-decoration: none;
	}
	.error {
		border: 2px solid black;
		padding: 0.5rem;
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
</style>
