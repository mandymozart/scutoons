<script lang="ts">
	import { page } from '$app/state';
	import { ActionBar, Badge, Button, DataTable, RecordInspector, Section, Skeleton, Toggle, type Column } from '@scutoons/ui';
	import type { AdminUserDetail, RoleInfo } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { day, errorText, minute } from '$lib/format.ts';
	import { inspectorLabels, tableLabels } from '$lib/labels.ts';
	import { getAdminSession } from '$lib/session.svelte.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PageHeader from '$lib/components/page-header.svelte';

	const toaster = getToaster();
	const session = getAdminSession();
	const id = $derived(page.params.id ?? '');

	let user = $state<AdminUserDetail | null>(null);
	let roles = $state<RoleInfo[]>([]);
	let error = $state<string | null>(null);
	let working = $state<string | null>(null);

	$effect(() => {
		const current = id;
		user = null;
		error = null;
		Promise.all([admin.getUser(current), admin.roles()])
			.then(([u, r]) => {
				user = u.user;
				roles = r.items;
			})
			.catch((e) => (error = errorText(e)));
	});

	const isSelf = $derived(!!user && user.email === session.user?.email);

	async function setRole(role: string, on: boolean) {
		if (!user) return;
		if (!on && role === 'admin' && isSelf && !confirm(m['users.revoke_own_admin']())) {
			user = { ...user };
			return;
		}
		working = role;
		try {
			const result = on ? await admin.grantRole(user.id, role) : await admin.revokeRole(user.id, role);
			user = result.user;
			toaster.addToast({
				message: on ? m['users.granted']({ role, email: user.email }) : m['users.revoked']({ role, email: user.email }),
				type: 'success'
			});
		} catch (e) {
			toaster.addToast({ message: errorText(e), type: 'error' });
			user = { ...user };
		} finally {
			working = null;
		}
	}

	const codeColumns = $derived<Column[]>([
		{ id: 'code', label: m['codes.col_code']() },
		{ id: 'type', label: m['codes.col_type']() },
		{ id: 'edition_id', label: m['common.edition']() },
		{ id: 'claimed_at', label: m['codes.col_claimed']() },
		{ id: 'revoked_at', label: m['codes.status_revoked']() }
	]);
</script>

{#if error}
	<PageHeader type={m['users.type_label']()} title={id} />
	<p class="error-box" role="alert">{error}</p>
{:else if !user}
	<PageHeader type={m['users.type_label']()} title={id} />
	<Skeleton lines={8} label={m['shell.loading']()} />
{:else}
	<PageHeader type={m['users.type_label']()} title={user.email} meta={m['users.meta']({ created: day(user.created_at), id: user.id })} />

	<ActionBar label={m['shell.page_actions']()} moreLabel={m['shell.more']()}>
		{#snippet start()}
			<a href="/users">← {m['users.back']()}</a>
		{/snippet}
	</ActionBar>

	<div class="grid-2">
		<Section title={m['users.section_account']()} hint={m['users.section_account_hint']()}>
			<dl class="facts">
				<dt>{m['users.col_signup_code']()}</dt>
				<dd>{#if user.signup_code}<a class="mono-code" href="/codes/{encodeURIComponent(user.signup_code)}">{user.signup_code}</a>{:else}—{/if}</dd>
				<dt>{m['users.col_opt_in']()}</dt>
				<dd>{user.marketing_opt_in ? `${m['common.yes']()} · ${minute(user.marketing_opt_in_at)}` : m['common.no']()}</dd>
				<dt>{m['users.locale']()}</dt>
				<dd>{user.locale ?? '—'}</dd>
				<dt>{m['users.inventory']()}</dt>
				<dd>{user.inventory_entries}</dd>
				<dt>{m['users.sessions']()}</dt>
				<dd>{user.sessions}</dd>
				<dt>{m['users.last_seen']()}</dt>
				<dd>{minute(user.last_seen_at)}</dd>
			</dl>
		</Section>
		<Section title={m['users.section_roles']()} hint={m['users.section_roles_hint']()}>
			<div class="roles">
				{#each roles as role (role.id)}
					<Toggle
						label={role.name}
						hint={role.description}
						checked={user.roles.includes(role.name)}
						disabled={working !== null}
						onchange={(on) => setRole(role.name, on)}
					/>
				{/each}
			</div>
			<p class="current">{#each user.roles as r (r)}<Badge tone={r === 'admin' ? 'ink' : 'neutral'}>{r}</Badge> {/each}</p>
		</Section>
	</div>

	<Section title={m['users.section_codes']()} hint={m['users.section_codes_hint']()}>
		<DataTable columns={codeColumns} rows={user.codes} rowKey={(c) => c.code} emptyLabel={m['users.no_codes']()} labels={tableLabels()}>
			{#snippet cell(c, column)}
				{#if column.id === 'code'}<a class="mono-code" href="/codes/{encodeURIComponent(c.code)}">{c.code}</a>
				{:else if column.id === 'edition_id'}{c.edition_id ?? '—'}{c.edition_number ? ` #${c.edition_number}` : ''}
				{:else if column.id === 'claimed_at'}{minute(c.claimed_at)}
				{:else if column.id === 'revoked_at'}{#if c.revoked_at}<Badge tone="error">{day(c.revoked_at)}</Badge>{:else}—{/if}
				{:else}{c.type}{/if}
			{/snippet}
		</DataTable>
	</Section>

	<Section title={m['codes.section_raw']()} hint={m['codes.section_raw_hint']()}>
		<RecordInspector record={user} labels={inspectorLabels()} />
	</Section>
{/if}

<style>
	.roles {
		display: flex;
		flex-direction: column;
		gap: var(--space-075);
	}
	.current {
		margin: var(--space-1) 0 0;
	}
</style>
