<script lang="ts">
	import { infoBar } from '$lib/info.ts';
	import PageHelp from '$lib/components/page-help.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { ActionBar, Badge, CloseIcon, DataTable, Pager, SearchField, Skeleton, listHref, listParams, readList, type Column } from '@scutoons/ui';
	import type { AdminClaimRow, ListResult } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { CLAIM_STATUSES, claimStatusLabel, claimTone, openClaims } from '$lib/claims.svelte.ts';
	import { day, errorText, minute, number } from '$lib/format.ts';
	import { pagerLabels, tableLabels } from '$lib/labels.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PageHeader from '$lib/components/page-header.svelte';
	import AccountLink from '$lib/components/account-link.svelte';

	/** Ownership claims (GET /admin/claims), URL-driven like the other lists: q, status, code, user. */
	const FILTERS = ['status', 'code', 'user'];
	const query = $derived(readList(page.url, { pageSize: 50, sort: '-created_at', filters: FILTERS }));

	let result = $state<(ListResult<AdminClaimRow> & { open: number }) | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(false);

	$effect(() => {
		const params = listParams(query);
		loading = true;
		error = null;
		admin
			.listClaims(params)
			.then((r) => {
				result = r;
				openClaims.set(r.open);
			})
			.catch((e) => (error = errorText(e)))
			.finally(() => (loading = false));
	});

	const columns = $derived<Column[]>([
		{ id: 'created_at', label: m['claims.col_filed'](), sortable: true },
		{ id: 'code', label: m['codes.col_code'](), sortable: true },
		{ id: 'status', label: m['claims.col_status'](), sortable: true },
		{ id: 'claimant', label: m['claims.col_claimant']() },
		{ id: 'owner', label: m['claims.col_owner']() },
		{ id: 'files', label: m['claims.col_files'](), align: 'end', secondary: true },
		{ id: 'statement', label: m['claims.col_statement'](), secondary: true },
		{ id: 'decided_at', label: m['claims.col_decided'](), sortable: true, secondary: true }
	]);

	const href = (patch: Record<string, string | number | null>) => listHref(page.url, patch);
	const nav = (patch: Record<string, string | number | null>) => goto(href(patch), { keepFocus: true, noScroll: true });

	/** the user filter shows the account's email when a row on the page has it */
	const userLabel = $derived.by(() => {
		const id = query.filters.user;
		if (!id) return '';
		for (const row of result?.items ?? []) {
			if (row.claimant.id === id && row.claimant.email) return row.claimant.email;
			if (row.owner?.id === id && row.owner.email) return row.owner.email;
		}
		return id;
	});
</script>

<PageHeader
	type={m['nav.group_codes']()}
	title={m['claims.title']()}
	meta={result ? m['claims.count']({ count: number(result.pagination.total), open: number(result.open) }) : undefined}
/>

{#snippet filters()}
	<SearchField compact label={m['claims.search']()} placeholder={m['claims.search_placeholder']()} value={query.q} onsearch={(q) => nav({ q })} />
	<label class="bar-filter">
		<span>{m['claims.col_status']()}</span>
		<select value={query.filters.status ?? ''} onchange={(e) => nav({ status: e.currentTarget.value })}>
			<option value="">{m['common.all']()}</option>
			{#each CLAIM_STATUSES as s (s)}
				<option value={s}>{claimStatusLabel(s)}</option>
			{/each}
		</select>
	</label>
	{#if query.filters.status !== 'open'}
		<a class="quick" href={href({ status: 'open' })} data-testid="claims-open-link">{m['claims.show_open']({ count: number(result?.open ?? openClaims.count ?? 0) })}</a>
	{/if}
	{#if query.filters.code}
		<span class="bar-filter">
			<span>{m['codes.col_code']()}</span>
			<a class="chip sc-chip-control" href={href({ code: null })} title={m['claims.remove_filter']()}>{query.filters.code} <CloseIcon size="var(--type-body-size)" /></a>
		</span>
	{/if}
	{#if query.filters.user}
		<span class="bar-filter">
			<span>{m['claims.filter_user']()}</span>
			<a class="chip sc-chip-control" href={href({ user: null })} title={m['claims.remove_filter']()}>{userLabel} <CloseIcon size="var(--type-body-size)" /></a>
		</span>
	{/if}
	{#if query.q || Object.keys(query.filters).length}
		<a class="quick" href="/claims">{m['common.reset_filters']()}</a>
	{/if}
{/snippet}

{#snippet barStart()}
	{#if result}
		<Pager compact page={result.pagination.page} pageSize={result.pagination.pageSize} total={result.pagination.total} href={(p) => href({ page: p })} labels={pagerLabels()} />
	{/if}
{/snippet}

<ActionBar start={barStart} label={m['shell.page_actions']()} moreLabel={m['shell.more']()} {...infoBar()} infoTitle={m['claims.title']()}>
	{#snippet info()}<PageHelp page="claims" />{/snippet}
</ActionBar>

{#if error}<p class="error-box" role="alert">{error}</p>{/if}

<div class="toolbar">{@render filters()}</div>

{#if !result}
	<Skeleton variant="table" columns={6} label={m['shell.loading']()} />
{:else}
	<DataTable
		{columns}
		rows={result.items}
		rowKey={(c) => c.id}
		sort={query.sort}
		sortHref={(sort) => href({ sort })}
		defaultSortDir={{ created_at: 'desc', decided_at: 'desc' }}
		onrowclick={(c) => goto(`/claims/${encodeURIComponent(c.id)}`)}
		{loading}
		labels={tableLabels()}
	>
		{#snippet cell(c, column)}
			{#if column.id === 'created_at'}<a class="nowrap" href="/claims/{encodeURIComponent(c.id)}" title={minute(c.created_at)}>{day(c.created_at)}</a>
			{:else if column.id === 'code'}<a class="mono-code" href="/codes/{encodeURIComponent(c.code)}">{c.code}</a>
				{#if c.edition_id}<span class="sub">{c.edition_id}{c.edition_number ? ` #${c.edition_number}` : ''}</span>{/if}
			{:else if column.id === 'status'}<Badge tone={claimTone(c.status)}>{claimStatusLabel(c.status)}</Badge>
			{:else if column.id === 'claimant'}<AccountLink account={c.claimant} />
			{:else if column.id === 'owner'}<AccountLink account={c.owner} />
				{#if c.owner_changed}<span class="sub">{m['claims.owner_changed_short']()}</span>{/if}
			{:else if column.id === 'files'}{c.files}
			{:else if column.id === 'statement'}<span class="muted statement">{c.statement}</span>
			{:else}{minute(c.decided_at)}{/if}
		{/snippet}
	</DataTable>
{/if}

<style>
	/* a removable filter = an interactive chip (tools.css .sc-chip-control: one control height, 2px ink) */
	.chip {
		gap: var(--space-05);
		padding-right: var(--space-05);
		text-decoration: none;
	}
	.nowrap {
		white-space: nowrap;
	}
	.quick {
		align-self: center;
	}
	.sub {
		display: block;
		overflow-wrap: anywhere;
		color: var(--text-muted);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.statement {
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
		max-width: 28rem;
	}
</style>
