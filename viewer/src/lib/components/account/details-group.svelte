<script lang="ts">
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { isMembershipExpired } from '@scutoons/content';
	import { SignedInAs } from '@scutoons/auth';
	import { getLocale, localizeHref } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { formatDate } from '$lib/utils/date.ts';
	import ReaderNameSetting from './reader-name-setting.svelte';
	import ShippingAddressSetting from './shipping-address-setting.svelte';

	/**
	 * Account page "Stammdaten" (owner 2026-10-16): the reader's name first (prominent – stories call the reader by it),
	 * then for members the address + sign out, roles, the memberships and the date the account was made, the shipping
	 * address (physical gifts of newsletters wait for it – #66), and "Delete
	 * account" – for now a link to the contact form with the privacy topic chosen (owner: "vorerst einmal einen Link auf
	 * das Kontaktformular mit der Option vorausgewählt"). Guests: the name only (kept on this device).
	 */
	const session = getSessionContext();
	const typeLabel = (type: string) => (m as any)[`account.${type}`]?.() ?? type;
	const roleLabel = (role: string) => (m as any)[`account.role_${role}`]?.() ?? role;
</script>

<ReaderNameSetting />

{#if session.isLoggedIn && session.user}
	<div class="signed-in">
		<SignedInAs session={session.auth} locale={getLocale()} />
	</div>
	<table>
		<tbody>
			{#if session.roles.length}
				<tr>
					<td>{m['account.roles']()}</td>
					<td>
						<span class="roles">
							{#each session.roles as role}<span class="role-tag">{roleLabel(role)}</span>{/each}
						</span>
					</td>
				</tr>
			{/if}
			{#each session.memberships as ms}
				<tr>
					<td>{typeLabel(ms.type)}</td>
					<td>
						{ms.code} · {ms.expires_at || m['account.lifetime']()}
						{#if isMembershipExpired(ms.expires_at)}(✕){/if}
					</td>
				</tr>
			{/each}
			{#if session.user.created_at}
				<tr>
					<td>{m['account.created_at']()}</td>
					<td>{formatDate(new Date(session.user.created_at.replace(' ', 'T')))}</td>
				</tr>
			{/if}
		</tbody>
	</table>
	<ShippingAddressSetting />
	<p class="delete">
		<a class="btn btn-secondary" href={localizeHref('/contact') + '?topic=privacy&from=account'} data-testid="delete-account">{m['account.delete_account']()}</a>
		<small>{m['account.delete_account_hint']()}</small>
	</p>
{/if}

<style>
	table {
		width: 100%;
		border-collapse: collapse;
	}
	td {
		padding: 0.25rem 0.5rem;
		vertical-align: top;
		overflow-wrap: anywhere;
	}
	td:first-child {
		background: var(--surface-hover);
		width: 40%;
	}
	.roles {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem;
	}
	.role-tag {
		display: inline-block;
		border: 2px solid var(--ink);
		padding: 0 0.375rem;
		font-size: 0.75rem;
		line-height: 1.4;
	}
	.delete {
		display: flex;
		align-items: center;
		gap: 0.5rem 1rem;
		flex-wrap: wrap;
		margin: 0;
	}
	.delete .btn {
		padding: 0 1rem;
		text-decoration: none;
	}
	small {
		font-size: 0.75rem;
		line-height: 1.3;
		color: var(--text-muted);
	}
</style>
