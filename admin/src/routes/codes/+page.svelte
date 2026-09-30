<script lang="ts">
	import { infoBar } from '$lib/info.ts';
	import PageHelp from '$lib/components/page-help.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import {
		ActionBar,
		Button,
		DataTable,
		Field,
		Pager,
		SearchField,
		Sheet,
		Skeleton,
		SplitView,
		listHref,
		listParams,
		readList,
		type Column
	} from '@scutoons/ui';
	import type { AdminCodeRow, ListResult } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { CODE_STATUSES, CODE_TYPES, loadCatalog, type Edition } from '$lib/catalog.ts';
	import { day, errorText, number } from '$lib/format.ts';
	import { pagerLabels, statusLabel, tableLabels } from '$lib/labels.ts';
	import { saveResponse } from '$lib/qr/download.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PageHeader from '$lib/components/page-header.svelte';
	import CodeStatus from '$lib/components/code-status.svelte';
	import CodeSummary from '$lib/components/code-summary.svelte';

	const toaster = getToaster();
	const FILTERS = ['edition_id', 'type', 'status'];

	const query = $derived(readList(page.url, { pageSize: 50, sort: '-created_at', filters: FILTERS }));
	const open = $derived(page.url.searchParams.get('open'));

	let result = $state<ListResult<AdminCodeRow> | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(false);
	let selected = $state<string[]>([]);
	let editions = $state<Edition[]>([]);
	let reload = $state(0);

	$effect(() => {
		loadCatalog()
			.then((c) => (editions = c.editions as Edition[]))
			.catch(() => {});
	});

	$effect(() => {
		const params = listParams(query);
		void reload;
		loading = true;
		error = null;
		admin
			.listCodes(params)
			.then((r) => (result = r))
			.catch((e) => (error = errorText(e)))
			.finally(() => (loading = false));
	});

	const columns = $derived<Column[]>([
		{ id: 'code', label: m['codes.col_code'](), sortable: true },
		{ id: 'type', label: m['codes.col_type'](), sortable: true, secondary: true },
		{ id: 'edition_id', label: m['common.edition'](), sortable: true },
		{ id: 'edition_number', label: '#', sortable: true, align: 'end', secondary: true },
		{ id: 'status', label: m['codes.col_status']() },
		{ id: 'scans', label: m['codes.col_scans'](), sortable: true, align: 'end' },
		{ id: 'devices', label: m['codes.col_devices'](), sortable: true, align: 'end', secondary: true },
		{ id: 'accesses', label: m['codes.col_unlocks'](), sortable: true, align: 'end', secondary: true },
		{ id: 'logins', label: m['codes.col_logins'](), sortable: true, align: 'end', secondary: true },
		{ id: 'refused', label: m['codes.col_refused'](), sortable: true, align: 'end', secondary: true },
		{ id: 'last_seen', label: m['codes.col_last_seen'](), sortable: true, secondary: true },
		{ id: 'created_at', label: m['codes.created'](), sortable: true }
	]);
	const countFields = ['scans', 'devices', 'accesses', 'logins', 'refused', 'edition_number', 'last_seen', 'created_at'];

	const href = (patch: Record<string, string | number | null>) => listHref(page.url, patch);
	const nav = (patch: Record<string, string | number | null>) => goto(href(patch), { keepFocus: true, noScroll: true });

	async function exportCsv() {
		try {
			const params = listParams(query);
			await saveResponse(await admin.codesCsv(params), `scutoons-codes-${new Date().toISOString().slice(0, 10)}.csv`);
		} catch (e) {
			toaster.addToast({ message: errorText(e), type: 'error' });
		}
	}

	// bulk revoke / unrevoke
	let bulkOpen = $state(false);
	let bulkAction = $state<'revoke' | 'unrevoke'>('revoke');
	let bulkReason = $state('');
	let bulkWorking = $state(false);

	function startBulk(action: 'revoke' | 'unrevoke') {
		bulkAction = action;
		bulkReason = '';
		bulkOpen = true;
	}

	async function runBulk() {
		bulkWorking = true;
		let ok = 0;
		const failed: string[] = [];
		for (const code of selected) {
			try {
				if (bulkAction === 'revoke') await admin.revokeCode(code, bulkReason.trim());
				else await admin.unrevokeCode(code);
				ok++;
			} catch {
				failed.push(code);
			}
		}
		bulkWorking = false;
		bulkOpen = false;
		toaster.addToast({
			message: m['codes.bulk_done']({ count: ok }) + (failed.length ? ` ${m['codes.bulk_failed']({ codes: failed.join(', ') })}` : ''),
			type: failed.length ? 'error' : 'success'
		});
		selected = failed;
		reload++;
	}
</script>

<PageHeader type={m['nav.group_codes']()} title={m['codes.title']()} meta={result ? m['codes.count']({ count: number(result.pagination.total) }) : undefined} />

{#snippet barStart()}
	<SearchField compact label={m['codes.search']()} placeholder={m['codes.search_placeholder']()} value={query.q} onsearch={(q) => nav({ q })} />
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
		<span>{m['codes.col_type']()}</span>
		<select value={query.filters.type ?? ''} onchange={(e) => nav({ type: e.currentTarget.value })}>
			<option value="">{m['common.all']()}</option>
			{#each CODE_TYPES as t (t)}
				<option value={t}>{t}</option>
			{/each}
		</select>
	</label>
	<label class="bar-filter">
		<span>{m['codes.col_status']()}</span>
		<select value={query.filters.status ?? ''} onchange={(e) => nav({ status: e.currentTarget.value })}>
			<option value="">{m['common.all']()}</option>
			{#each CODE_STATUSES as s (s)}
				<option value={s}>{statusLabel(s)}</option>
			{/each}
		</select>
	</label>
	{#if query.q || Object.keys(query.filters).length}
		<a class="reset" href="/codes">{m['common.reset_filters']()}</a>
	{/if}
	{#if result}
		<Pager
			compact
			page={result.pagination.page}
			pageSize={result.pagination.pageSize}
			total={result.pagination.total}
			href={(p) => href({ page: p })}
			labels={pagerLabels()}
		/>
	{/if}
{/snippet}

<ActionBar start={barStart} label={m['shell.page_actions']()} moreLabel={m['shell.more']()} {...infoBar()} infoTitle={m['codes.title']()}>
	{#snippet info()}<PageHelp page="codes" />{/snippet}
	{#snippet secondary()}
		<Button size="sm" variant="secondary" onclick={exportCsv}>{m['codes.export_csv']()}</Button>
	{/snippet}
	{#snippet end()}
		<Button size="sm" href="/codes/new">{m['nav.generate']()}</Button>
	{/snippet}
</ActionBar>

{#if error}
	<p class="error-box" role="alert">{error}</p>
{/if}

<SplitView
	open={!!open}
	title={open ?? ''}
	closeLabel={m['common.close']()}
	onclose={() => nav({ open: null, page: query.page > 1 ? query.page : null })}
>
	{#snippet list()}
		{#if !result}
			<Skeleton variant="table" columns={6} label={m['shell.loading']()} />
		{:else}
			<DataTable
				{columns}
				rows={result.items}
				rowKey={(row) => row.code}
				sort={query.sort}
				sortHref={(sort) => href({ sort })}
				defaultSortDir={Object.fromEntries(countFields.map((f) => [f, 'desc' as const]))}
				selectable
				bind:selected
				activeKey={open}
				rowClass={(row) => (row.revoked_at ? 'revoked' : '')}
				onrowclick={(row) => nav({ open: row.code, page: query.page > 1 ? query.page : null })}
				{loading}
				emptyLabel={m['codes.empty']()}
				labels={tableLabels()}
			>
				{#snippet cell(row, column)}
					{#if column.id === 'code'}
						<a class="mono-code" href="/codes/{encodeURIComponent(row.code)}">{row.code}</a>
					{:else if column.id === 'status'}
						<CodeStatus claimed={row.claimed} revoked={!!row.revoked_at} signed={row.signed} />
					{:else if column.id === 'type'}
						{row.type}
					{:else if column.id === 'edition_id'}
						{row.edition_id ?? '—'}
					{:else if column.id === 'created_at' || column.id === 'last_seen'}
						{day(row[column.id])}
					{:else if column.id === 'edition_number'}
						{row.edition_number ?? '—'}
					{:else}
						{number(row[column.id as keyof AdminCodeRow] as number)}
					{/if}
				{/snippet}
				{#snippet bulk()}
					<Button size="sm" variant="secondary" onclick={() => startBulk('revoke')}>{m['codes.revoke']()}</Button>
					<Button size="sm" variant="secondary" onclick={() => startBulk('unrevoke')}>{m['codes.unrevoke']()}</Button>
				{/snippet}
			</DataTable>
		{/if}
	{/snippet}
	{#snippet actions()}
		{#if open}<Button size="sm" variant="secondary" href="/codes/{encodeURIComponent(open)}">{m['codes.open_page']()}</Button>{/if}
	{/snippet}
	{#snippet detail()}
		{#if open}<CodeSummary code={open} onchanged={() => reload++} />{/if}
	{/snippet}
</SplitView>

<Sheet bind:open={bulkOpen} title={bulkAction === 'revoke' ? m['codes.bulk_revoke_title']({ count: selected.length }) : m['codes.bulk_unrevoke_title']({ count: selected.length })} closeLabel={m['common.close']()}>
	<p>{selected.join(', ')}</p>
	{#if bulkAction === 'revoke'}
		<Field label={m['codes.reason']()} hint={m['codes.reason_hint']()}>
			{#snippet children({ id, describedby })}
				<input {id} aria-describedby={describedby} maxlength="255" bind:value={bulkReason} />
			{/snippet}
		</Field>
	{/if}
	{#snippet footer()}
		<Button size="sm" variant="secondary" onclick={() => (bulkOpen = false)}>{m['common.cancel']()}</Button>
		<Button size="sm" disabled={bulkWorking} onclick={runBulk}>
			{bulkAction === 'revoke' ? m['codes.revoke']() : m['codes.unrevoke']()}
		</Button>
	{/snippet}
</Sheet>

<style>
	.reset {
		align-self: center;
	}
	:global(tr.revoked td) {
		color: var(--text-muted);
	}
</style>
