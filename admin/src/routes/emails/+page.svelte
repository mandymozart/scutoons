<script lang="ts">
	import { infoBar } from '$lib/info.ts';
	import PageHelp from '$lib/components/page-help.svelte';
	import EmailsNav from '$lib/components/emails-nav.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { ActionBar, Badge, CloseIcon, DataTable, Pager, SearchField, Skeleton, listHref, listParams, readList, type Column } from '@scutoons/ui';
	import type { AdminEmailLogRow, ListResult } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { errorText, number } from '$lib/format.ts';
	import { pagerLabels, tableLabels } from '$lib/labels.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PageHeader from '$lib/components/page-header.svelte';

	/** The sent-mail log of the API's email module (GET /api/admin/emails/log), URL-driven like the other lists. */
	const FILTERS = ['template', 'status', 'recipient', 'from', 'to'];
	const query = $derived(readList(page.url, { pageSize: 50, sort: '-created_at', filters: FILTERS }));

	let result = $state<ListResult<AdminEmailLogRow> | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(false);
	let templates = $state<string[]>([]);

	$effect(() => {
		admin
			.emailTemplates()
			.then((r) => (templates = r.items.map((t) => t.id)))
			.catch(() => {});
	});

	$effect(() => {
		const params = listParams(query);
		loading = true;
		error = null;
		admin
			.listEmails(params)
			.then((r) => (result = r))
			.catch((e) => (error = errorText(e)))
			.finally(() => (loading = false));
	});

	const columns = $derived<Column[]>([
		{ id: 'created_at', label: m['emails.col_when'](), sortable: true },
		{ id: 'template', label: m['emails.col_template'](), sortable: true },
		{ id: 'recipient', label: m['emails.col_recipient'](), sortable: true },
		{ id: 'status', label: m['emails.col_status'](), sortable: true },
		{ id: 'locale', label: m['emails.col_locale'](), secondary: true },
		{ id: 'transport', label: m['emails.col_transport'](), secondary: true },
		{ id: 'error', label: m['emails.col_error']() }
	]);

	const statusLabel = (status: string) => (status === 'sent' ? m['emails.status_sent']() : status === 'failed' ? m['emails.status_failed']() : status);
	const href = (patch: Record<string, string | number | null>) => listHref(page.url, patch);
	const nav = (patch: Record<string, string | number | null>) => goto(href(patch), { keepFocus: true, noScroll: true });
</script>

<PageHeader type={m['nav.group_system']()} title={m['emails.title']()} meta={result ? m['emails.count']({ count: number(result.pagination.total) }) : undefined} />
<EmailsNav current="log" />

{#snippet barStart()}
	<SearchField compact label={m['emails.search']()} placeholder={m['emails.search_placeholder']()} value={query.q} onsearch={(q) => nav({ q })} />
	<label class="bar-filter">
		<span>{m['emails.template']()}</span>
		<select value={query.filters.template ?? ''} onchange={(e) => nav({ template: e.currentTarget.value })}>
			<option value="">{m['common.all']()}</option>
			{#each templates as id (id)}
				<option value={id}>{id}</option>
			{/each}
		</select>
	</label>
	<label class="bar-filter">
		<span>{m['emails.status']()}</span>
		<select value={query.filters.status ?? ''} onchange={(e) => nav({ status: e.currentTarget.value })}>
			<option value="">{m['common.all']()}</option>
			<option value="sent">{m['emails.status_sent']()}</option>
			<option value="failed">{m['emails.status_failed']()}</option>
		</select>
	</label>
	<label class="bar-filter">
		<span>{m['emails.from']()}</span>
		<input type="date" value={query.filters.from ?? ''} onchange={(e) => nav({ from: e.currentTarget.value })} />
	</label>
	<label class="bar-filter">
		<span>{m['emails.to']()}</span>
		<input type="date" value={query.filters.to ?? ''} onchange={(e) => nav({ to: e.currentTarget.value })} />
	</label>
	{#if query.filters.recipient}
		<span class="bar-filter">
			<span>{m['emails.col_recipient']()}</span>
			<a class="chip sc-chip-control" href={href({ recipient: null })} title={m['emails.remove_filter']()}>{query.filters.recipient} <CloseIcon size="var(--type-body-size)" /></a>
		</span>
	{/if}
	{#if query.q || Object.keys(query.filters).length}
		<a href="/emails">{m['common.reset_filters']()}</a>
	{/if}
	{#if result}
		<Pager compact page={result.pagination.page} pageSize={result.pagination.pageSize} total={result.pagination.total} href={(p) => href({ page: p })} labels={pagerLabels()} />
	{/if}
{/snippet}

<ActionBar start={barStart} label={m['shell.page_actions']()} moreLabel={m['shell.more']()} {...infoBar()} infoTitle={m['emails.title']()}>
	{#snippet info()}<PageHelp page="emails" />{/snippet}
</ActionBar>

{#if error}<p class="error-box" role="alert">{error}</p>{/if}

{#if !result}
	<Skeleton variant="table" columns={5} label={m['shell.loading']()} />
{:else}
	<DataTable
		{columns}
		rows={result.items}
		rowKey={(e) => String(e.id)}
		sort={query.sort}
		sortHref={(sort) => href({ sort })}
		{loading}
		labels={tableLabels()}
	>
		{#snippet cell(e, column)}
			{#if column.id === 'created_at'}{e.created_at.replace('T', ' ').replace(/\+00:00|Z$/, '')}
			{:else if column.id === 'template'}<a href={href({ template: e.template })}>{e.template}</a>
			{:else if column.id === 'recipient'}{e.recipient}
				<a class="filter-link" href={href({ recipient: e.recipient })} title={m['emails.filter_recipient']()} aria-label={m['emails.filter_recipient']()}>⌕</a>
				<a class="filter-link" href="/users?q={encodeURIComponent(e.recipient)}" title={m['emails.open_user']()} aria-label={m['emails.open_user']()}>→</a>
			{:else if column.id === 'status'}<Badge tone={e.status === 'failed' ? 'error' : 'neutral'}>{statusLabel(e.status)}</Badge>
			{:else if column.id === 'error'}<span class="error-text">{e.error ?? '—'}</span>
			{:else}{e[column.id as 'locale' | 'transport']}{/if}
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
	.filter-link {
		margin-left: var(--space-025);
		text-decoration: none;
	}
	.error-text {
		overflow-wrap: anywhere;
	}
</style>
