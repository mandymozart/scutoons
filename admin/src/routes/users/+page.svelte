<script lang="ts">
	import { infoBar } from '$lib/info.ts';
	import PageHelp from '$lib/components/page-help.svelte';
	import AccountFilters from '$lib/components/account-filters.svelte';
	import SegmentsSheet from '$lib/components/segments-sheet.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { ActionBar, Badge, Button, DataTable, Pager, SearchField, Skeleton, confirmDialog, listHref, listParams, readList, type Column } from '@scutoons/ui';
	import type { AccountFilterParams, AdminSegment, AdminUserRow, ListResult, RoleInfo } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { campaignError } from '$lib/campaigns.ts';
	import { day, errorText, number } from '$lib/format.ts';
	import { pagerLabels, tableLabels } from '$lib/labels.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PageHeader from '$lib/components/page-header.svelte';

	/**
	 * The accounts: search, the account filters (agents/research/GIFT-INVITES.md 1b – signed up with, edition, batch,
	 * campaign, segment), each one's segments; tick accounts → add to / remove from a segment, or add every account the
	 * filters find ("all matching", every page). Segments are made in the Segments sheet.
	 */
	const FILTERS = ['role', 'opt_in', 'signup', 'edition', 'batch', 'campaign', 'campaign_role', 'segment'];
	const query = $derived(readList(page.url, { pageSize: 50, sort: '-created_at', filters: FILTERS }));
	const toaster = getToaster();

	let result = $state<ListResult<AdminUserRow> | null>(null);
	let roles = $state<RoleInfo[]>([]);
	let segments = $state<AdminSegment[]>([]);
	let error = $state<string | null>(null);
	let loading = $state(false);
	let selected = $state<string[]>([]);
	let target = $state('');
	let managing = $state(false);
	let reload = $state(0);

	$effect(() => {
		admin
			.roles()
			.then((r) => (roles = r.items))
			.catch(() => {});
	});

	function loadSegments() {
		admin
			.segments()
			.then((r) => {
				segments = r.items;
				if (target && !segments.some((s) => s.id === target)) target = '';
			})
			.catch(() => {});
	}
	loadSegments();

	$effect(() => {
		const params = listParams(query);
		void reload;
		loading = true;
		error = null;
		admin
			.listUsers(params)
			.then((r) => (result = r))
			.catch((e) => (error = errorText(e)))
			.finally(() => (loading = false));
	});

	const filtered = $derived(!!query.q || Object.keys(query.filters).length > 0);
	const targetName = $derived(segments.find((s) => s.id === target)?.name ?? '');

	async function changeSegment(action: 'add' | 'remove', ids: string[]) {
		if (!target) return;
		try {
			const count = action === 'add' ? (await admin.addToSegment(target, { ids })).added : (await admin.removeFromSegment(target, ids)).removed;
			toaster.addToast({ message: (action === 'add' ? m['segments.added'] : m['segments.removed'])({ count: number(count), name: targetName }), type: 'success' });
			selected = [];
			loadSegments();
			reload++;
		} catch (e) {
			toaster.addToast({ message: campaignError(e), type: 'error' });
		}
	}

	/** every account the search and filters find, on every page */
	async function addMatching() {
		if (!target || !result) return;
		const ok = await confirmDialog({
			title: m['segments.add_matching_title']({ count: number(result.pagination.total), name: targetName }),
			message: m['segments.add_matching_message'](),
			confirmLabel: m['segments.add_matching']()
		});
		if (!ok) return;
		try {
			const r = await admin.addToSegment(target, { matching: true, q: query.q || undefined, filters: query.filters as AccountFilterParams });
			toaster.addToast({ message: m['segments.added']({ count: number(r.added), name: targetName }), type: 'success' });
			loadSegments();
			reload++;
		} catch (e) {
			toaster.addToast({ message: campaignError(e), type: 'error' });
		}
	}

	const columns = $derived<Column[]>([
		{ id: 'email', label: m['users.col_email'](), sortable: true },
		{ id: 'created_at', label: m['users.col_created'](), sortable: true },
		{ id: 'roles', label: m['users.col_roles']() },
		{ id: 'segments', label: m['users.col_segments']() },
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
	<AccountFilters filters={query.filters} onchange={nav} />
	{#if result}
		<Pager compact page={result.pagination.page} pageSize={result.pagination.pageSize} total={result.pagination.total} href={(p) => href({ page: p })} labels={pagerLabels()} />
	{/if}
{/snippet}

<ActionBar start={barStart} label={m['shell.page_actions']()} moreLabel={m['shell.more']()} {...infoBar()} infoTitle={m['users.title']()}>
	{#snippet end()}
		<Button size="sm" variant="secondary" onclick={() => (managing = true)}>{m['segments.title']()}</Button>
	{/snippet}
	{#snippet info()}<PageHelp page="users" />{/snippet}
</ActionBar>

{#if error}<p class="error-box" role="alert">{error}</p>{/if}

{#snippet segmentPicker()}
	<label class="bar-filter">
		<span>{m['segments.segment']()}</span>
		<select bind:value={target}>
			<option value="" disabled>{m['segments.choose']()}</option>
			{#each segments as s (s.id)}<option value={s.id}>{s.name}</option>{/each}
		</select>
	</label>
{/snippet}

{#if filtered && result && result.pagination.total > 0 && segments.length > 0}
	<div class="matching">
		<span>{m['segments.matching_line']({ count: number(result.pagination.total) })}</span>
		{@render segmentPicker()}
		<Button size="sm" variant="secondary" disabled={!target} onclick={addMatching}>{m['segments.add_matching']()}</Button>
	</div>
{/if}

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
		selectable={segments.length > 0}
		bind:selected
		{loading}
		labels={tableLabels()}
	>
		{#snippet bulk(keys)}
			{@render segmentPicker()}
			<Button size="sm" variant="secondary" disabled={!target} onclick={() => changeSegment('add', keys)}>{m['segments.add']()}</Button>
			<Button size="sm" variant="secondary" disabled={!target} onclick={() => changeSegment('remove', keys)}>{m['segments.remove']()}</Button>
		{/snippet}
		{#snippet cell(u, column)}
			{#if column.id === 'email'}
				<a href="/users/{encodeURIComponent(u.id)}">{u.email}</a>
			{:else if column.id === 'created_at'}
				{day(u.created_at)}
			{:else if column.id === 'roles'}
				<span class="roles">
					{#each u.roles as r (r)}<Badge tone={r === 'admin' ? 'ink' : 'neutral'}>{r}</Badge>{/each}
				</span>
			{:else if column.id === 'segments'}
				<span class="roles">
					{#each u.segments ?? [] as s (s.id)}<Badge tone="info">{s.name}</Badge>{/each}
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

<SegmentsSheet bind:open={managing} {segments} onchange={() => (loadSegments(), reload++)} />

<style>
	.roles {
		display: inline-flex;
		flex-wrap: wrap;
		gap: var(--space-025);
	}
	/* "N accounts match – add them all to a segment" (every page) */
	.matching {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-1);
		margin: 0 0 var(--space-1);
	}
</style>
