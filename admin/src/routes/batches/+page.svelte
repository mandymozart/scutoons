<script lang="ts">
	import { infoBar } from '$lib/info.ts';
	import PageHelp from '$lib/components/page-help.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { ActionBar, Badge, Button, DataTable, Field, Pager, Sheet, Skeleton, listHref, listParams, readList, type Column } from '@scutoons/ui';
	import { ApiError, type BatchRow, type ListResult } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { downloadPrintSheet, downloadZip } from '$lib/batch-actions.ts';
	import { loadCatalog, type Edition } from '$lib/catalog.ts';
	import { day, errorText, number } from '$lib/format.ts';
	import { pagerLabels, tableLabels } from '$lib/labels.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PageHeader from '$lib/components/page-header.svelte';

	const toaster = getToaster();
	const query = $derived(readList(page.url, { pageSize: 50, sort: '-created_at', filters: ['edition_id', 'complete'] }));

	let result = $state<ListResult<BatchRow> | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(false);
	let editions = $state<Edition[]>([]);
	let busy = $state<string | null>(null);

	$effect(() => {
		loadCatalog()
			.then((c) => (editions = c.editions as Edition[]))
			.catch(() => {});
	});

	$effect(() => {
		const params = listParams(query);
		loading = true;
		admin
			.listBatches(params)
			.then((r) => (result = r))
			.catch((e) => (error = errorText(e)))
			.finally(() => (loading = false));
	});

	const columns = $derived<Column[]>([
		{ id: 'created_at', label: m['batches.col_date'](), sortable: true },
		{ id: 'edition', label: m['common.edition'](), sortable: true },
		{ id: 'done', label: m['batches.col_progress'](), sortable: true, align: 'end' },
		{ id: 'errors', label: m['batches.col_errors'](), sortable: true, align: 'end' },
		{ id: 'complete', label: m['codes.col_status']() },
		{ id: 'actions', label: '' }
	]);

	const href = (patch: Record<string, string | number | null>) => listHref(page.url, patch);

	// new batch (`?new=1` opens the form, also from the command palette)
	const newOpen = $derived(page.url.searchParams.get('new') === '1');
	let sheetOpen = $state(false);
	$effect.pre(() => {
		sheetOpen = newOpen;
	});
	let editionId = $state('');
	let count = $state<number | null>(null);
	let startNumber = $state(1);
	let completeWarning = $state<string | null>(null);
	let creating = $state(false);
	let createError = $state<string | null>(null);

	const edition = $derived(editions.find((e) => e.id === editionId));
	$effect(() => {
		if (!editionId && editions.length) editionId = editions[0].id;
	});
	$effect(() => {
		// the count defaults to the edition size; a complete batch of the edition needs a deliberate "anyway"
		if (edition) count = edition.edition_count;
		completeWarning = null;
	});

	function closeForm() {
		goto(listHref(page.url, { new: null, page: query.page > 1 ? query.page : null }), { keepFocus: true, noScroll: true });
	}

	async function create(force = false) {
		if (!edition) return;
		creating = true;
		createError = null;
		try {
			const { batch } = await admin.createBatch({
				edition_id: edition.id,
				count: count || edition.edition_count,
				start_number: startNumber || 1,
				force
			});
			await goto(`/batches/${encodeURIComponent(batch.filename)}?run=1`);
		} catch (e) {
			if (e instanceof ApiError && e.code === 'batch_complete') {
				const b = e.body?.batch;
				completeWarning = m['batches.complete_warning']({
					done: b?.done ?? b?.codes?.filter((c: { status: string }) => c.status === 'done').length ?? '?',
					date: day(b?.created_at)
				});
			} else createError = errorText(e);
		} finally {
			creating = false;
		}
	}

	async function run(key: string, action: () => Promise<void>) {
		busy = key;
		try {
			await action();
		} catch (e) {
			toaster.addToast({ message: errorText(e), type: 'error' });
		} finally {
			busy = null;
		}
	}
</script>

<PageHeader type={m['nav.group_codes']()} title={m['batches.title']()} meta={m['batches.meta']()} />

{#snippet barStart()}
	<label class="bar-filter">
		<span>{m['common.edition']()}</span>
		<select value={query.filters.edition_id ?? ''} onchange={(e) => goto(href({ edition_id: e.currentTarget.value }))}>
			<option value="">{m['common.all']()}</option>
			{#each editions as e (e.id)}
				<option value={e.id}>{e.name}</option>
			{/each}
		</select>
	</label>
	<label class="bar-filter">
		<span>{m['codes.col_status']()}</span>
		<select value={query.filters.complete ?? ''} onchange={(e) => goto(href({ complete: e.currentTarget.value }))}>
			<option value="">{m['common.all']()}</option>
			<option value="1">{m['batches.complete']()}</option>
			<option value="0">{m['batches.incomplete']()}</option>
		</select>
	</label>
	{#if result}
		<Pager compact page={result.pagination.page} pageSize={result.pagination.pageSize} total={result.pagination.total} href={(p) => href({ page: p })} labels={pagerLabels()} />
	{/if}
{/snippet}

<ActionBar start={barStart} label={m['shell.page_actions']()} moreLabel={m['shell.more']()} {...infoBar()} infoTitle={m['batches.title']()}>
	{#snippet info()}<PageHelp page="batches" />{/snippet}
	{#snippet end()}
		<Button size="sm" href={href({ new: 1, page: query.page > 1 ? query.page : null })}>{m['batches.new']()}</Button>
	{/snippet}
</ActionBar>

{#if error}<p class="error-box" role="alert">{error}</p>{/if}

{#if !result}
	<Skeleton variant="table" columns={5} label={m['shell.loading']()} />
{:else}
	<DataTable
		{columns}
		rows={result.items}
		rowKey={(b) => b.filename}
		sort={query.sort}
		sortHref={(sort) => href({ sort })}
		{loading}
		labels={tableLabels()}
	>
		{#snippet cell(b, column)}
			{#if column.id === 'created_at'}
				<a href="/batches/{encodeURIComponent(b.filename)}">{day(b.created_at)}</a>
				<span class="file muted">{b.filename}</span>
			{:else if column.id === 'edition'}
				{b.edition}
			{:else if column.id === 'done'}
				{number(b.done)} / {number(b.total)}
			{:else if column.id === 'errors'}
				{#if b.errors > 0}<Badge tone="error">{b.errors}</Badge>{:else}—{/if}
			{:else if column.id === 'complete'}
				{#if b.complete}<Badge tone="ink">{m['batches.complete']()}</Badge>{:else}<Badge tone="warning">{m['batches.incomplete']()}</Badge>{/if}
			{:else if column.id === 'actions'}
				<span class="row-actions">
					{#if !b.complete}
						<Button size="sm" href="/batches/{encodeURIComponent(b.filename)}?run=1">{m['batches.resume']()}</Button>
					{/if}
					{#if b.done > 0}
						<Button size="sm" variant="secondary" disabled={busy === `zip-${b.filename}`} onclick={() => run(`zip-${b.filename}`, () => downloadZip(b.filename))}>
							{m['batches.zip']()}
						</Button>
						<Button size="sm" variant="secondary" disabled={busy === `print-${b.filename}`} onclick={() => run(`print-${b.filename}`, () => downloadPrintSheet(b.filename))}>
							{busy === `print-${b.filename}` ? m['batches.generating']() : m['batches.print_sheet']()}
						</Button>
					{/if}
				</span>
			{/if}
		{/snippet}
	</DataTable>
{/if}

<Sheet bind:open={sheetOpen} side="right" title={m['batches.new']()} closeLabel={m['common.close']()} onclose={() => newOpen && closeForm()}>
	<form id="new-batch" onsubmit={(e) => (e.preventDefault(), create(false))}>
		<Field label={m['common.edition']()}>
			{#snippet children({ id })}
				<select {id} bind:value={editionId}>
					{#each editions as e (e.id)}
						<option value={e.id}>{m['batches.edition_option']({ name: e.name, count: e.edition_count })}</option>
					{/each}
				</select>
			{/snippet}
		</Field>
		{#if edition}
			<dl class="facts">
				<dt>{m['batches.product']()}</dt>
				<dd>{edition.name}</dd>
				<dt>{m['batches.excluded']()}</dt>
				<dd>{edition.excluded_number ?? 0}</dd>
				<dt>{m['batches.creators']()}</dt>
				<dd class="pre">{edition.creators ?? '—'}</dd>
			</dl>
		{/if}
		<Field label={m['batches.count']()} hint={m['batches.count_hint']()}>
			{#snippet children({ id, describedby })}
				<input {id} type="number" min="1" max="2000" aria-describedby={describedby} bind:value={count} />
			{/snippet}
		</Field>
		<Field label={m['batches.start_number']()}>
			{#snippet children({ id })}
				<input {id} type="number" min="1" bind:value={startNumber} />
			{/snippet}
		</Field>
		{#if completeWarning}
			<div class="error-box" role="alert">
				<p>{completeWarning}</p>
				<Button size="sm" variant="secondary" disabled={creating} onclick={() => create(true)}>{m['batches.create_anyway']()}</Button>
			</div>
		{/if}
		{#if createError}<p class="error-box" role="alert">{createError}</p>{/if}
	</form>
	{#snippet footer()}
		<Button size="sm" variant="secondary" onclick={closeForm}>{m['common.cancel']()}</Button>
		<Button size="sm" type="submit" form="new-batch" disabled={creating || !edition}>{m['batches.start']()}</Button>
	{/snippet}
</Sheet>

<style>
	.file {
		display: block;
		font-size: var(--type-caption-size);
	}
	.row-actions {
		display: flex;
		flex-wrap: nowrap;
		gap: var(--space-025);
		justify-content: flex-end;
	}
	.pre {
		white-space: pre-line;
	}
	.facts {
		margin-bottom: var(--space-1) !important;
	}
	.error-box p {
		margin: 0 0 var(--space-05);
	}
</style>
