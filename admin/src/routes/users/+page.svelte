<script lang="ts">
	import { infoBar } from '$lib/info.ts';
	import PageHelp from '$lib/components/page-help.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { ActionBar, Badge, DataTable, Pager, SearchField, Skeleton, listHref, listParams, readList, type Column } from '@scutoons/ui';
	import type { AdminUserRow, ListResult, RoleInfo } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { day, errorText, number } from '$lib/format.ts';
	import { pagerLabels, tableLabels } from '$lib/labels.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PageHeader from '$lib/components/page-header.svelte';

	const query = $derived(readList(page.url, { pageSize: 50, sort: '-created_at', filters: ['role', 'opt_in'] }));

	let result = $state<ListResult<AdminUserRow> | null>(null);
	let roles = $state<RoleInfo[]>([]);
	let error = $state<string | null>(null);
	let loading = $state(false);

	$effect(() => {
		admin
			.roles()
			.then((r) => (roles = r.items))
			.catch(() => {});
	});

	$effect(() => {
		const params = listParams(query);
		loading = true;
		error = null;
		admin
			.listUsers(params)
			.then((r) => (result = r))
			.catch((e) => (error = errorText(e)))
			.finally(() => (loading = false));
	});

	const columns = $derived<Column[]>([
		{ id: 'email', label: m['users.col_email'](), sortable: true },
		{ id: 'created_at', label: m['users.col_created'](), sortable: true },
		{ id: 'roles', label: m['users.col_roles']() },
		{ id: 'signup_code', label: m['users.col_signup_code'](), secondary: true },
		{ id: 'marketing_opt_in', label: m['users.col_opt_in'](), secondary: true },
		{ id: 'codes', label: m['users.col_codes'](), sortable: true, align: 'end' }
	]);

	const href = (patch: Record<string, string | number | null>) => listHref(page.url, patch);
	const nav = (patch: Record<string, string | number | null>) => goto(href(patch), { keepFocus: true, noScroll: true });
</script>

<PageHeader type={m['nav.group_people']()} title={m['users.title']()} meta={result ? m['users.count']({ count: number(result.pagination.total) }) : undefined} />

{#snippet barStart()}
	<SearchField compact label={m['users.search']()} placeholder={m['users.search_placeholder']()} value={query.q} onsearch={(q) => nav({ q })} />
	<label class="bar-filter">
		<span>{m['users.col_roles']()}</span>
		<select value={query.filters.role ?? ''} onchange={(e) => nav({ role: e.currentTarget.value })}>
			<option value="">{m['common.all']()}</option>
			{#each roles as r (r.id)}
				<option value={r.name}>{r.name}</option>
			{/each}
		</select>
	</label>
	<label class="bar-filter">
		<span>{m['users.col_opt_in']()}</span>
		<select value={query.filters.opt_in ?? ''} onchange={(e) => nav({ opt_in: e.currentTarget.value })}>
			<option value="">{m['common.all']()}</option>
			<option value="1">{m['common.yes']()}</option>
			<option value="0">{m['common.no']()}</option>
		</select>
	</label>
	{#if result}
		<Pager compact page={result.pagination.page} pageSize={result.pagination.pageSize} total={result.pagination.total} href={(p) => href({ page: p })} labels={pagerLabels()} />
	{/if}
{/snippet}

<ActionBar start={barStart} label={m['shell.page_actions']()} moreLabel={m['shell.more']()} {...infoBar()} infoTitle={m['users.title']()}>
	{#snippet info()}<PageHelp page="users" />{/snippet}
</ActionBar>

{#if error}<p class="error-box" role="alert">{error}</p>{/if}

{#if !result}
	<Skeleton variant="table" columns={5} label={m['shell.loading']()} />
{:else}
	<DataTable
		{columns}
		rows={result.items}
		rowKey={(u) => u.id}
		sort={query.sort}
		sortHref={(sort) => href({ sort })}
		defaultSortDir={{ codes: 'desc', created_at: 'desc' }}
		onrowclick={(u) => goto(`/users/${encodeURIComponent(u.id)}`)}
		{loading}
		emptyLabel={m['users.empty']()}
		labels={tableLabels()}
	>
		{#snippet cell(u, column)}
			{#if column.id === 'email'}
				<a href="/users/{encodeURIComponent(u.id)}">{u.email}</a>
			{:else if column.id === 'created_at'}
				{day(u.created_at)}
			{:else if column.id === 'roles'}
				<span class="roles">
					{#each u.roles as r (r)}<Badge tone={r === 'admin' ? 'ink' : 'neutral'}>{r}</Badge>{/each}
				</span>
			{:else if column.id === 'signup_code'}
				{#if u.signup_code}<a class="mono-code" href="/codes/{encodeURIComponent(u.signup_code)}">{u.signup_code}</a>{:else}—{/if}
			{:else if column.id === 'marketing_opt_in'}
				{u.marketing_opt_in ? `${m['common.yes']()} ${day(u.marketing_opt_in_at)}` : '—'}
			{:else}
				{number(u.codes)}
			{/if}
		{/snippet}
	</DataTable>
{/if}

<style>
	.roles {
		display: inline-flex;
		flex-wrap: wrap;
		gap: var(--space-025);
	}
</style>
