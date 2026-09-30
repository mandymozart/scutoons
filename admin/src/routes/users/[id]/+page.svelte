<script lang="ts">
	import { infoBar } from '$lib/info.ts';
	import PageHelp from '$lib/components/page-help.svelte';
	import { page } from '$app/state';
	import { ActionBar, Badge, Button, DataTable, Field, RecordInspector, Section, Skeleton, Toggle, type Column } from '@scutoons/ui';
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

	// ── Avatar-body unlocks (ITEMS.md "Reader avatar bodies & character unlocks") ──
	let unlockCharacter = $state('');
	let unlockLooks = $state('');

	async function unlock(e: SubmitEvent) {
		e.preventDefault();
		if (!user || !unlockCharacter.trim()) return;
		const character = unlockCharacter.trim();
		const looks = unlockLooks.split(',').map((l) => l.trim()).filter(Boolean);
		working = 'characters';
		try {
			const result = await admin.unlockCharacter(user.id, character, looks.length ? { looks } : {});
			user = { ...user, characters: result.characters };
			unlockCharacter = '';
			unlockLooks = '';
			toaster.addToast({ message: m['users.unlocked']({ character, email: user.email }), type: 'success' });
		} catch (err) {
			toaster.addToast({ message: errorText(err), type: 'error' });
		} finally {
			working = null;
		}
	}

	async function lock(character: string) {
		if (!user) return;
		working = 'characters';
		try {
			const result = await admin.lockCharacter(user.id, character);
			user = { ...user, characters: result.characters };
			toaster.addToast({ message: m['users.unlock_revoked']({ character, email: user.email }), type: 'success' });
		} catch (err) {
			toaster.addToast({ message: errorText(err), type: 'error' });
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

	<ActionBar label={m['shell.page_actions']()} moreLabel={m['shell.more']()} {...infoBar()} infoTitle={user.email}>
		{#snippet info()}<PageHelp page="user" />{/snippet}
		{#snippet start()}
			<a href="/users">← {m['users.back']()}</a>
		{/snippet}
	</ActionBar>

	<div class="grid-2">
		<Section title={m['users.section_account']()}>
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
		<Section title={m['users.section_roles']()}>
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

	<Section title={m['users.section_characters']()}>
		<ul class="unlocks" data-testid="character-unlocks">
			{#each user.characters ?? [] as u (u.character)}
				<li>
					<strong>{u.character}</strong>
					{#each u.looks as look (look)}<Badge tone="neutral">{look}</Badge> {/each}
					<span class="meta">{u.source}{u.ref ? ` · ${u.ref}` : ''}{u.unlocked_at ? ` · ${minute(u.unlocked_at)}` : ''}</span>
					{#if u.source !== 'default'}
						<Button variant="clear" size="sm" disabled={working !== null} onclick={() => lock(u.character)}>{m['users.unlock_revoke']()}</Button>
					{/if}
				</li>
			{/each}
		</ul>
		<form class="unlock" onsubmit={unlock}>
			<Field label={m['users.character_id']()}>
				{#snippet children({ id, describedby, invalid })}
					<input {id} aria-describedby={describedby} aria-invalid={invalid} bind:value={unlockCharacter} placeholder="yellowmouse" data-testid="unlock-character" />
				{/snippet}
			</Field>
			<Field label={m['users.character_looks']()}>
				{#snippet children({ id, describedby, invalid })}
					<input {id} aria-describedby={describedby} aria-invalid={invalid} bind:value={unlockLooks} placeholder="naked" />
				{/snippet}
			</Field>
			<Button type="submit" disabled={working !== null || !unlockCharacter.trim()}>{m['users.unlock']()}</Button>
		</form>
	</Section>

	<Section title={m['users.section_codes']()}>
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

	<Section title={m['codes.section_raw']()}>
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
	.unlocks {
		list-style: none;
		margin: 0 0 var(--space-1);
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
	}
	.unlocks li {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-05);
	}
	.unlocks .meta {
		font-size: 0.85em;
		opacity: 0.7;
	}
	.unlock {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-end;
		gap: var(--space-1);
	}
</style>
