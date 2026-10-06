<script lang="ts">
	import { infoBar } from '$lib/info.ts';
	import PageHelp from '$lib/components/page-help.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { ActionBar, CloseIcon, DataTable, Pager, SearchField, Skeleton, listHref, listParams, readList, type Column } from '@scutoons/ui';
	import { EVENT_NAMES, type AdminEventRow, type ListResult } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { loadCatalog, type Edition } from '$lib/catalog.ts';
	import { errorText, number } from '$lib/format.ts';
	import { eventLabel, pagerLabels, tableLabels } from '$lib/labels.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PageHeader from '$lib/components/page-header.svelte';

	const FILTERS = ['code', 'event', 'edition_id', 'toon', 'from', 'to'];
	const query = $derived(readList(page.url, { pageSize: 100, sort: '-created_at', filters: FILTERS }));
	const activeEvents = $derived(query.filters.event ? query.filters.event.split(',') : []);

	let result = $state<ListResult<AdminEventRow> | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(false);
	let editions = $state<Edition[]>([]);
	let toons = $state<{ id: string; name: string }[]>([]);

	$effect(() => {
		loadCatalog()
			.then((c) => {
				editions = c.editions as Edition[];
				toons = c.toons;
			})
			.catch(() => {});
	});

	$effect(() => {
		const params = listParams(query);
		loading = true;
		error = null;
		admin
			.listEvents(params)
			.then((r) => (result = r))
			.catch((e) => (error = errorText(e)))
			.finally(() => (loading = false));
	});

	const columns = $derived<Column[]>([
		{ id: 'created_at', label: m['events.col_when'](), sortable: true },
		{ id: 'event', label: m['events.col_event'](), sortable: true },
		{ id: 'code', label: m['codes.col_code'](), sortable: true },
		{ id: 'edition_id', label: m['common.edition'](), secondary: true },
		{ id: 'toon', label: m['events.col_toon'](), secondary: true },
		{ id: 'device', label: m['events.col_device'](), secondary: true },
		{ id: 'account', label: m['events.col_account']() }
	]);

	const href = (patch: Record<string, string | number | null>) => listHref(page.url, patch);
	const nav = (patch: Record<string, string | number | null>) => goto(href(patch), { keepFocus: true, noScroll: true });

	function toggleEvent(name: string) {
		const next = activeEvents.includes(name) ? activeEvents.filter((e) => e !== name) : [...activeEvents, name];
		nav({ event: next.join(',') });
	}
</script>

<PageHeader type={m['nav.group_people']()} title={m['events.title']()} meta={result ? m['events.count']({ count: number(result.pagination.total) }) : undefined} />

{#snippet barStart()}
	<SearchField compact label={m['events.search']()} placeholder={m['codes.search_placeholder']()} value={query.q} onsearch={(q) => nav({ q })} />
	<label class="bar-filter">
		<span>{m['common.edition']()}</span>
		<select value={query.filters.edition_id ?? ''} onchange={(e) => nav({ edition_id: e.currentTarget.value })}>
			<option value="">{m['common.all']()}</option>
			<option value="none">{m['common.no_edition']()}</option>
			{#each editions as e (e.id)}
				<option value={e.id}>{e.name}</option>
			{/each}
		</select>
	</label>
	<label class="bar-filter">
		<span>{m['events.col_toon']()}</span>
		<select value={query.filters.toon ?? ''} onchange={(e) => nav({ toon: e.currentTarget.value })}>
			<option value="">{m['common.all']()}</option>
			{#each toons as t (t.id)}
				<option value={t.id}>{t.name}</option>
			{/each}
		</select>
	</label>
	<label class="bar-filter">
		<span>{m['events.from']()}</span>
		<input type="date" value={query.filters.from ?? ''} onchange={(e) => nav({ from: e.currentTarget.value })} />
	</label>
	<label class="bar-filter">
		<span>{m['events.to']()}</span>
		<input type="date" value={query.filters.to ?? ''} onchange={(e) => nav({ to: e.currentTarget.value })} />
	</label>
	{#if query.filters.code}
		<span class="bar-filter">
			<span>{m['codes.col_code']()}</span>
			<a class="chip sc-chip-control" href={href({ code: null })} title={m['events.remove_filter']()}>{query.filters.code} <CloseIcon size="var(--type-body-size)" /></a>
		</span>
	{/if}
	<details class="dropup bar-filter">
		<summary><span>{m['events.col_event']()}</span> {activeEvents.length ? activeEvents.map(eventLabel).join(', ') : m['common.all']()}</summary>
		<div class="dropup-panel">
			{#each EVENT_NAMES as name (name)}
				<label><input type="checkbox" checked={activeEvents.includes(name)} onchange={() => toggleEvent(name)} /> {eventLabel(name)}</label>
			{/each}
		</div>
	</details>
	{#if query.q || Object.keys(query.filters).length}
		<a href="/events">{m['common.reset_filters']()}</a>
	{/if}
	{#if result}
		<Pager compact page={result.pagination.page} pageSize={result.pagination.pageSize} total={result.pagination.total} href={(p) => href({ page: p })} labels={pagerLabels()} />
	{/if}
{/snippet}

<ActionBar start={barStart} label={m['shell.page_actions']()} moreLabel={m['shell.more']()} {...infoBar()} infoTitle={m['events.title']()}>
	{#snippet info()}<PageHelp page="events" />{/snippet}
</ActionBar>


{#if error}<p class="error-box" role="alert">{error}</p>{/if}

{#if !result}
	<Skeleton variant="table" columns={6} label={m['shell.loading']()} />
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
			{#if column.id === 'event'}{eventLabel(e.event)}
			{:else if column.id === 'code'}<a class="mono-code" href="/codes/{encodeURIComponent(e.code)}">{e.code}</a>
				<a class="filter-link" href={href({ code: e.code })} title={m['events.filter_code']()}>⌕</a>
			{:else if column.id === 'account'}{e.account ? m['common.yes']() : '—'}
			{:else if column.id === 'created_at'}{e.created_at.replace('T', ' ').replace(/\+00:00|Z$/, '')}
			{:else}{e[column.id as 'edition_id' | 'toon' | 'device'] ?? '—'}{/if}
		{/snippet}
	</DataTable>
{/if}

<style>
	/* fold-ok: a dropdown's trigger (a control), not a fold row */
	.dropup summary {
		font-size: var(--type-body-size);
		line-height: var(--type-body-line);
		cursor: pointer;
		padding: var(--space-025) var(--space-05);
		border: var(--border-width-hairline) solid var(--border-subtle);
		list-style: none;
		max-width: 16rem;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.dropup summary span {
		color: var(--text-muted);
		font-size: var(--type-caption-size);
	}
	.dropup[open] summary {
		border-color: var(--ink);
	}
	/* fixed: escapes the bar's horizontal scroll container, opens upwards above the bar */
	.dropup-panel {
		position: fixed;
		z-index: 50;
		bottom: calc(var(--action-bar-height) + var(--space-05));
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		padding: var(--space-075) var(--space-1);
		background: var(--paper);
		border: var(--border-width-regular) solid var(--ink);
	}
	.dropup-panel label {
		display: inline-flex;
		align-items: center;
		gap: var(--space-05);
		cursor: pointer;
	}

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
</style>
