<script lang="ts">
	import { Badge, Button, BulkProgress, BulkRun, DataTable, Section, Skeleton, chunk, confirmDialog, listHref, listParams, type Column, type ListState } from '@scutoons/ui';
	import type { AssetCleanupResult, AssetUnused, AssetUnusedList } from '@scutoons/api';
	import { page } from '$app/state';
	import { assets } from '$lib/api.ts';
	import { bytes, errorText, number } from '$lib/format.ts';
	import { tableLabels } from '$lib/labels.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * Admin → Assets → Unused (agents/ASSETS.md "cleanup", research/ASSET-LISTS.md): the files nothing references as a
	 * server-side list – filters and the pager in the page's bar (search, folder, kind, what goes, map tiles), sort; a map tile
	 * set is ONE row. Tick rows or "Select all N" (every page), then one confirm and the delete runs in batches with a
	 * progress bar and Cancel; the report follows. The first batch rebuilds the usage index, every batch re-checks.
	 */
	let {
		query,
		reload = 0,
		result = $bindable(null),
		onchange
	}: {
		/** the page's list state (q, folder, kind, what, tiles, sort, page) */
		query: ListState;
		/** bumped by the page (Rescan): the next load rebuilds the usage index */
		reload?: number;
		result?: AssetUnusedList | null;
		/** after a delete (the library's counts change) */
		onchange?: () => void;
	} = $props();

	/** files per request: bunny answers each delete + purge on its own, a request must end well inside the host's time limit */
	const BATCH_FILES = 25;
	const FAILED_SHOWN = 20;

	let error = $state<string | null>(null);
	let loading = $state(false);
	let selected = $state<string[]>([]);
	let allMatching = $state(false);
	/** file count + size of every row seen (a selection spans pages) */
	let seen = $state<Record<string, { files: number; size: number }>>({});
	let fresh = true;
	let lastReload = -1;
	let again = $state(0);
	const run = new BulkRun<string[], AssetCleanupResult>((batch) => batch.length);
	let report = $state<{ deleted: number; bytes: number; assets: number; skipped: number; failed: [string, string][]; done: number; total: number; cancelled: boolean; error: string | null } | null>(null);

	const params = $derived.by(() => {
		const p = listParams(query);
		delete p.view;
		delete p.asset;
		delete p.state;
		return p;
	});
	/** the filters without the page: a change ends the selection */
	const filterKey = $derived(JSON.stringify({ ...params, page: undefined, page_size: undefined, sort: undefined }));
	$effect(() => {
		void filterKey;
		selected = [];
		allMatching = false;
	});

	$effect(() => {
		void again;
		if (reload !== lastReload) {
			fresh = true;
			lastReload = reload;
		}
		const ask = { ...params, fresh: fresh ? 1 : undefined };
		loading = true;
		error = null;
		assets
			.unused(ask)
			.then((r) => {
				fresh = false;
				result = r;
				for (const row of r.items) seen[row.id] = { files: row.file_count, size: row.size };
			})
			.catch((e) => (error = errorText(e)))
			.finally(() => (loading = false));
	});

	const chosenFiles = $derived(allMatching ? (result?.matching.files ?? 0) : selected.reduce((n, id) => n + (seen[id]?.files ?? 0), 0));
	const chosenBytes = $derived(allMatching ? (result?.matching.bytes ?? 0) : selected.reduce((n, id) => n + (seen[id]?.size ?? 0), 0));

	/** the delete units: an asset's files stay together (all of them → the whole asset goes), a tile set splits per tile */
	function units(rows: AssetUnused[]): string[][] {
		return rows.flatMap((r) => (r.tiles !== null ? r.files.map((f) => [f.path]) : [r.files.map((f) => f.path)]));
	}

	async function cleanup() {
		error = null;
		report = null;
		let rows: AssetUnused[];
		try {
			// every file of the chosen rows (a page carries only a few of a tile set's)
			const all = await assets.unused({ ...params, page: undefined, page_size: undefined, all: 1 });
			rows = allMatching ? all.items : all.items.filter((r) => selected.includes(r.id));
		} catch (e) {
			error = errorText(e);
			return;
		}
		const batches = chunk(units(rows), BATCH_FILES, (u) => u.length).map((b) => b.flat());
		const files = batches.reduce((n, b) => n + b.length, 0);
		if (!files) return;
		const ok = await confirmDialog({
			title: m['assets.cleanup_title']({ files: number(files) }),
			message: m['assets.cleanup_message']({ size: bytes(rows.reduce((n, r) => n + r.size, 0)) }),
			confirmLabel: m['assets.cleanup_confirm']()
		});
		if (!ok) return;
		const done = await run.start(batches, (paths, i) => assets.cleanup(paths, i === 0), errorText);
		report = {
			deleted: done.results.reduce((n, r) => n + r.deleted.length, 0),
			bytes: done.results.reduce((n, r) => n + r.bytes, 0),
			assets: done.results.reduce((n, r) => n + r.assets_deleted, 0),
			skipped: done.results.reduce((n, r) => n + r.skipped.length, 0),
			failed: done.results.flatMap((r) => Object.entries(r.failed)),
			done: done.done,
			total: done.total,
			cancelled: done.cancelled,
			error: done.error
		};
		selected = [];
		allMatching = false;
		again++;
		onchange?.();
	}

	const columns = $derived<Column[]>([
		{ id: 'slug', label: m['assets.col_asset'](), sortable: true },
		{ id: 'kind', label: m['assets.col_kind']() },
		{ id: 'files', label: m['assets.col_what'](), sortable: true },
		{ id: 'size', label: m['assets.col_size'](), align: 'end', sortable: true }
	]);
	const href = (patch: Record<string, string | number | null>) => listHref(page.url, patch);
	const kindLabel = (k: string) => (m as unknown as Record<string, () => string>)[`assets.kind_${k}`]?.() ?? k;
	const whatLabel = (w: string) => (m as unknown as Record<string, () => string>)[`assets.what_${w}`]?.() ?? w;
	const filtered = $derived(!!query.q || ['folder', 'kind', 'what', 'tiles'].some((f) => query.filters[f]));
</script>

<p class="sc-caption">{m['assets.unused_hint']()}</p>
{#if error}<p class="error-box" role="alert">{error}</p>{/if}

<BulkProgress {run} label={m['assets.cleanup_running']()} count={(done, total) => m['assets.cleanup_progress']({ done: number(done), total: number(total) })} cancelLabel={m['common.cancel']()}>
	{#if report}
		<Section title={m['assets.cleanup_report']()}>
			<p data-testid="assets-cleanup-report">
				{m['assets.cleanup_done']({ files: number(report.deleted), size: bytes(report.bytes), assets: number(report.assets) })}
				{#if report.skipped}{' '}{m['assets.cleanup_skipped']({ count: number(report.skipped) })}{/if}
			</p>
			{#if report.error}<p class="error-box" role="alert">{m['assets.cleanup_stopped']({ done: number(report.done), total: number(report.total), error: report.error })}</p>
			{:else if report.cancelled}<p>{m['assets.cleanup_cancelled']({ done: number(report.done), total: number(report.total) })}</p>{/if}
			{#if report.failed.length}
				<p class="sc-caption">{m['assets.cleanup_failed_list']()}</p>
				<ul class="refs">
					{#each report.failed.slice(0, FAILED_SHOWN) as [path, why] (path)}<li><code>{path}</code> – {why}</li>{/each}
					{#if report.failed.length > FAILED_SHOWN}<li class="sc-caption">+{number(report.failed.length - FAILED_SHOWN)}</li>{/if}
				</ul>
			{/if}
		</Section>
	{/if}
</BulkProgress>

{#if !result}
	<Skeleton variant="table" columns={4} label={m['shell.loading']()} />
{:else}
	{#if result.totals.files}
		<p data-testid="assets-unused-totals">
			{m['assets.unused_totals']({ assets: number(result.totals.assets), files: number(result.totals.files), size: bytes(result.totals.bytes) })}
			{#if filtered}<br /><span class="sc-caption">{m['assets.unused_matching']({ rows: number(result.matching.rows), files: number(result.matching.files), size: bytes(result.matching.bytes) })}</span>{/if}
		</p>
	{/if}
	<DataTable
		{columns}
		rows={result.items}
		rowKey={(r) => r.id}
		sort={query.sort}
		sortHref={(sort) => href({ sort })}
		defaultSortDir={{ size: 'desc', files: 'desc' }}
		selectable
		bind:selected
		bind:allMatching
		total={result.pagination.total}
		{loading}
		labels={tableLabels()}
	>
		{#snippet bulk()}
			<Button size="sm" disabled={run.running} onclick={cleanup} data-testid="assets-cleanup">{m['assets.cleanup_delete']({ files: number(chosenFiles), size: bytes(chosenBytes) })}</Button>
		{/snippet}
		{#snippet cell(r, column)}
			{#if column.id === 'slug'}
				{#if r.tiles !== null}<span title={m['assets.tile_set_hint']()}>{r.slug}</span>{:else}<a href={href({ asset: r.id })}>{r.slug}</a>{/if}
			{:else if column.id === 'kind'}{kindLabel(r.kind)}
			{:else if column.id === 'files'}
				{#if r.tiles !== null}<Badge tone="warning">{m['assets.tile_set']({ tiles: number(r.tiles) })}</Badge>
				{:else}
					{#if r.all_unused}<Badge tone="warning">{whatLabel('asset')}</Badge>{/if}
					{#each r.files as f (f.path)}<span class="ref" title={f.path}>{f.path.split('/').pop()}{#if !r.all_unused || f.what !== 'current'} <Badge tone="neutral">{whatLabel(f.what)}</Badge>{/if}</span>{/each}
					{#if r.file_count > r.files.length}<span class="sc-caption">+{number(r.file_count - r.files.length)}</span>{/if}
				{/if}
			{:else}{bytes(r.size)}{/if}
		{/snippet}
	</DataTable>
{/if}

<style>
	.refs {
		list-style: none;
		margin: 0 0 var(--space-1);
		padding: 0;
	}
	.ref {
		display: inline-flex;
		gap: var(--space-025);
		align-items: center;
		margin-right: var(--space-05);
	}
</style>
