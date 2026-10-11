<script lang="ts">
	import { infoBar } from '$lib/info.ts';
	import PageHelp from '$lib/components/page-help.svelte';
	import PageHeader from '$lib/components/page-header.svelte';
	import AssetDetail from './asset-detail.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { ActionBar, Badge, Button, DataTable, Icon, Pager, SearchField, Section, Sheet, Skeleton, Tabs, confirmDialog, listHref, listParams, readList, type Column } from '@scutoons/ui';
	import { ASSET_KINDS, type AssetKind, type AssetList, type AssetMissing, type AssetStorageStatus, type AssetSummary, type AssetUnused } from '@scutoons/api';
	import { assets } from '$lib/api.ts';
	import { bytes, errorText, number } from '$lib/format.ts';
	import { auditAssets, type StandardFinding } from '@scutoons/media';
	import { pagerLabels, tableLabels } from '$lib/labels.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The asset manager (agents/ASSETS.md): the media registry over the CDN storage – a library of every asset (current
	 * version, renditions, uses), references to missing files, unused files, and the storage (drivers, bunny.net
	 * settings status, bandwidth, optimization jobs). The asset's detail (replace, rename, relink, optimize, delete with
	 * the usage check) opens in a sheet. URL-driven like the other lists.
	 */
	const FILTERS = ['folder', 'kind', 'state', 'view', 'asset'];
	const query = $derived(readList(page.url, { pageSize: 60, sort: 'slug', filters: FILTERS }));
	const view = $derived(query.filters.view ?? 'library');

	let list = $state<AssetList | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(false);
	let missing = $state<AssetMissing[] | null>(null);
	let unused = $state<AssetUnused[] | null>(null);
	let unusedTotals = $state<{ assets: number; files: number; bytes: number } | null>(null);
	let selected = $state<string[]>([]);
	let storage = $state<AssetStorageStatus | null>(null);
	/** the optimisation tool (BACKLOG 10.64): the assets off the media standard – DRAFT, listing only (owner to decide) */
	let standard = $state<StandardFinding[] | null>(null);
	let standardPicked = $state<string[]>([]);
	let busy = $state('');
	let notice = $state('');
	let reload = $state(0);

	$effect(() => {
		void reload;
		if (view !== 'library') return;
		const params = listParams(query);
		delete params.view;
		delete params.asset;
		loading = true;
		error = null;
		assets
			.list(params)
			.then((r) => (list = r))
			.catch((e) => (error = errorText(e)))
			.finally(() => (loading = false));
	});
	$effect(() => {
		void reload;
		if (view === 'missing') assets.missing().then((r) => (missing = r.items)).catch((e) => (error = errorText(e)));
		if (view === 'unused')
			assets
				.unused()
				.then((r) => ((unused = r.items), (unusedTotals = r.totals), (selected = selected.filter((id) => r.items.some((i) => i.id === id)))))
				.catch((e) => (error = errorText(e)));
		if (view === 'storage') assets.storage().then((r) => (storage = r)).catch((e) => (error = errorText(e)));
		if (view === 'standard') loadStandard().catch((e) => (error = errorText(e)));
	});
	/** every image / video asset (all pages), audited against the media standard in the browser (media-audit.ts) */
	async function loadStandard() {
		const all: AssetSummary[] = [];
		for (const kind of ['image', 'video']) {
			for (let p = 1; ; p++) {
				const r = await assets.list({ kind, page: p, page_size: 100 });
				all.push(...r.items);
				if (p * r.pagination.pageSize >= r.pagination.total) break;
			}
		}
		standard = auditAssets(all);
	}

	const tabs = $derived([
		{ id: 'library', label: m['assets.tab_library']() },
		{ id: 'missing', label: m['assets.tab_missing']() },
		{ id: 'unused', label: m['assets.tab_unused']() },
		{ id: 'standard', label: m['assets.tab_standard']() },
		{ id: 'storage', label: m['assets.tab_storage']() }
	]);
	const href = (patch: Record<string, string | number | null>) => listHref(page.url, patch);
	const nav = (patch: Record<string, string | number | null>) => goto(href(patch), { keepFocus: true, noScroll: true });

	const cdn = $derived((list?.cdn ?? storage?.public_base ?? '').replace(/\/+$/, ''));
	const url = (path: string) => `${cdn}/${path}`;
	function thumb(a: AssetSummary): string {
		if (a.kind === 'image') return [...a.renditions].filter((r) => r.mime?.startsWith('image/')).sort((x, y) => x.size - y.size)[0]?.path ?? a.path;
		return a.renditions.find((r) => r.label === 'poster')?.path ?? '';
	}
	const icon = (k: AssetKind) => ({ image: 'image', video: 'film', audio: 'music', archive: 'archive', document: 'file-text', other: 'file' })[k];
	const kindLabel = (k: string) => (m as unknown as Record<string, () => string>)[`assets.kind_${k}`]?.() ?? k;

	async function rescan() {
		busy = 'scan';
		notice = '';
		try {
			const r = await assets.scan();
			notice = m['assets.scanned']({ files: number(r.import.files), created: number(r.import.assets_created), usages: number(r.usages.content + r.usages.catalog), missing: number(r.usages.missing) });
			reload++;
		} catch (e) {
			error = errorText(e);
		} finally {
			busy = '';
		}
	}
	async function loadBandwidth() {
		busy = 'bandwidth';
		try {
			storage = await assets.storage(true);
		} catch (e) {
			error = errorText(e);
		} finally {
			busy = '';
		}
	}

	const missingColumns = $derived<Column[]>([
		{ id: 'path', label: m['assets.col_path']() },
		{ id: 'refs', label: m['assets.col_used_by']() }
	]);
	const unusedColumns = $derived<Column[]>([
		{ id: 'slug', label: m['assets.col_asset']() },
		{ id: 'kind', label: m['assets.col_kind']() },
		{ id: 'files', label: m['assets.col_what']() },
		{ id: 'size', label: m['assets.col_size'](), align: 'end' }
	]);
	const standardColumns = $derived<Column[]>([
		{ id: 'path', label: m['assets.col_path']() },
		{ id: 'advice', label: m['assets.col_off_standard']() },
		{ id: 'fix', label: m['assets.col_fix']() },
		{ id: 'usages', label: m['assets.col_used_by'](), align: 'end' },
		{ id: 'size', label: m['assets.col_size'](), align: 'end' }
	]);
	const standardChosen = $derived((standard ?? []).filter((f) => standardPicked.includes(f.path) && f.fix));
	const whatLabel = (w: string) => (m as unknown as Record<string, () => string>)[`assets.what_${w}`]?.() ?? w;
	const chosen = $derived((unused ?? []).filter((i) => selected.includes(i.id)));
	const chosenFiles = $derived(chosen.reduce((n, i) => n + i.files.length, 0));
	const chosenBytes = $derived(chosen.reduce((n, i) => n + i.size, 0));

	/** deletes every listed file of the selected assets; the server re-checks each path against a fresh usage index */
	async function cleanup() {
		const items = chosen;
		const files = items.flatMap((i) => i.files.map((f) => f.path));
		if (!files.length) return;
		const ok = await confirmDialog({
			title: m['assets.cleanup_title']({ files: number(files.length) }),
			message: m['assets.cleanup_message']({ size: bytes(chosenBytes) }),
			confirmLabel: m['assets.cleanup_confirm']()
		});
		if (!ok) return;
		busy = 'cleanup';
		error = null;
		notice = '';
		try {
			const r = await assets.cleanup(files);
			const failed = Object.entries(r.failed);
			notice = [
				m['assets.cleanup_done']({ files: number(r.deleted.length), size: bytes(r.bytes), assets: number(r.assets_deleted) }),
				r.skipped.length ? m['assets.cleanup_skipped']({ count: number(r.skipped.length) }) : ''
			]
				.filter(Boolean)
				.join(' ');
			if (failed.length) error = m['assets.cleanup_failed']({ count: number(failed.length), first: `${failed[0][0]}: ${failed[0][1]}` });
			selected = [];
			reload++;
		} catch (e) {
			error = errorText(e);
		} finally {
			busy = '';
		}
	}
	const settingColumns = $derived<Column[]>([
		{ id: 'name', label: m['assets.col_setting']() },
		{ id: 'status', label: m['assets.col_status']() },
		{ id: 'value', label: m['assets.col_value']() },
		{ id: 'note', label: m['assets.col_note']() }
	]);
	const settingTone = (s: string) => (s === 'set' ? 'published' : s === 'invalid' ? 'error' : 'warning');
	const detailOpen = $derived(!!query.filters.asset);
</script>

<PageHeader
	type={m['nav.group_data']()}
	title={m['assets.title']()}
	meta={list ? m['assets.count']({ count: number(list.pagination.total) }) : undefined}
/>

{#snippet barStart()}
	{#if view === 'library'}
		<SearchField compact label={m['assets.search']()} placeholder={m['assets.search_placeholder']()} value={query.q} onsearch={(q) => nav({ q })} />
		<label class="bar-filter">
			<span>{m['assets.folder']()}</span>
			<select value={query.filters.folder ?? ''} onchange={(e) => nav({ folder: e.currentTarget.value, page: null })} data-testid="assets-folder">
				<option value="">{m['common.all']()}</option>
				{#each list?.folders ?? [] as f (f.folder ?? '')}<option value={f.folder ?? ''}>{f.folder ?? '/'} ({f.assets})</option>{/each}
			</select>
		</label>
		<label class="bar-filter">
			<span>{m['assets.kind']()}</span>
			<select value={query.filters.kind ?? ''} onchange={(e) => nav({ kind: e.currentTarget.value, page: null })} data-testid="assets-kind">
				<option value="">{m['common.all']()}</option>
				{#each ASSET_KINDS as k (k)}<option value={k}>{kindLabel(k)}</option>{/each}
			</select>
		</label>
		<label class="bar-filter">
			<span>{m['assets.state']()}</span>
			<select value={query.filters.state ?? ''} onchange={(e) => nav({ state: e.currentTarget.value, page: null })}>
				<option value="">{m['common.all']()}</option>
				<option value="used">{m['assets.state_used']()}</option>
				<option value="unused">{m['assets.state_unused']()}</option>
			</select>
		</label>
		{#if list}
			<Pager compact page={list.pagination.page} pageSize={list.pagination.pageSize} total={list.pagination.total} href={(p) => href({ page: p })} labels={pagerLabels()} />
		{/if}
	{/if}
{/snippet}
{#snippet barPrimary()}
	<Button size="sm" variant="secondary" disabled={busy === 'scan'} onclick={rescan} title={m['assets.rescan_hint']()} data-testid="assets-rescan">{busy === 'scan' ? '…' : m['assets.rescan']()}</Button>
{/snippet}

<ActionBar start={barStart} end={barPrimary} label={m['shell.page_actions']()} moreLabel={m['shell.more']()} {...infoBar()} infoTitle={m['assets.title']()}>
	{#snippet info()}<PageHelp page="assets" />{/snippet}
</ActionBar>

{#if error}<p class="error-box" role="alert">{error}</p>{/if}
{#if notice}<p class="notice" role="status">{notice}</p>{/if}

<Tabs {tabs} value={view} label={m['assets.title']()} onchange={(id) => nav({ view: id === 'library' ? null : id, page: null })}>
	{#snippet children(active)}
		{#if active === 'library'}
			{#if !list}
				<Skeleton variant="table" columns={4} label={m['shell.loading']()} />
			{:else if !list.items.length}
				<p class="sc-empty">{m['assets.empty']()}</p>
			{:else}
				<ul class="grid" aria-busy={loading} data-testid="assets-grid">
					{#each list.items as a (a.id)}
						{@const src = thumb(a)}
						<li>
							<a class="card" href={href({ asset: a.id })} title={`${a.slug}\n${a.renditions.map((r) => r.label).join(' · ')}`} data-testid="asset-card">
								<span class="img">{#if src}<img src={url(src)} alt="" loading="lazy" />{:else}<Icon name={icon(a.kind)} size="2em" />{/if}</span>
								<span class="name">{a.path.split('/').pop()}</span>
								<span class="sc-caption line">{a.folder ?? '/'} · {bytes(a.size)}{a.width && a.height ? ` · ${a.width}×${a.height}` : ''}</span>
								<span class="badges">
									{#if a.usages}<Badge tone="neutral">{m['assets.uses']({ count: a.usages })}</Badge>{:else}<Badge tone="warning">{m['assets.state_unused']()}</Badge>{/if}
									{#if a.renditions.length > 1}<Badge tone="info">{m['assets.files']({ count: a.renditions.length })}</Badge>{/if}
								</span>
							</a>
						</li>
					{/each}
				</ul>
			{/if}
		{:else if active === 'missing'}
			<p class="sc-caption">{m['assets.missing_hint']()}</p>
			{#if !missing}
				<Skeleton variant="table" columns={2} label={m['shell.loading']()} />
			{:else}
				<DataTable columns={missingColumns} rows={missing} rowKey={(r) => r.path} labels={tableLabels()}>
					{#snippet cell(r, column)}
						{#if column.id === 'path'}<code>{r.path}</code>
						{:else}
							{#each r.usages.slice(0, 6) as u, i (i)}<span class="ref">{u.ref} <Badge tone={u.state === 'published' || u.state === 'config' ? 'error' : 'draft'}>{u.state}</Badge></span>{/each}
							{#if r.usages.length > 6}<span class="sc-caption">+{r.usages.length - 6}</span>{/if}
						{/if}
					{/snippet}
				</DataTable>
			{/if}
		{:else if active === 'unused'}
			<p class="sc-caption">{m['assets.unused_hint']()}</p>
			{#if !unused}
				<Skeleton variant="table" columns={4} label={m['shell.loading']()} />
			{:else}
				{#if unusedTotals && unusedTotals.files}<p data-testid="assets-unused-totals">{m['assets.unused_totals']({ assets: number(unusedTotals.assets), files: number(unusedTotals.files), size: bytes(unusedTotals.bytes) })}</p>{/if}
				<DataTable columns={unusedColumns} rows={unused} rowKey={(r) => r.id} selectable bind:selected labels={tableLabels()}>
					{#snippet bulk()}
						<Button size="sm" disabled={busy === 'cleanup'} onclick={cleanup} data-testid="assets-cleanup">{busy === 'cleanup' ? '…' : m['assets.cleanup_delete']({ files: number(chosenFiles), size: bytes(chosenBytes) })}</Button>
					{/snippet}
					{#snippet cell(r, column)}
						{#if column.id === 'slug'}<a href={href({ asset: r.id })}>{r.slug}</a>
						{:else if column.id === 'kind'}{kindLabel(r.kind)}
						{:else if column.id === 'files'}
							{#if r.all_unused}<Badge tone="warning">{whatLabel('asset')}</Badge>{/if}
							{#each r.files as f (f.path)}<span class="ref" title={f.path}>{f.path.split('/').pop()}{#if !r.all_unused || f.what !== 'current'} <Badge tone="neutral">{whatLabel(f.what)}</Badge>{/if}</span>{/each}
						{:else}{bytes(r.size)}{/if}
					{/snippet}
				</DataTable>
			{/if}
		{:else if active === 'standard'}
			<p class="sc-caption">{m['assets.standard_hint']()}</p>
			{#if !standard}
				<Skeleton variant="table" columns={5} label={m['shell.loading']()} />
			{:else}
				{#if standard.length}<p data-testid="assets-standard-totals">{m['assets.standard_totals']({ files: number(standard.length), size: bytes(standard.reduce((n, f) => n + f.size, 0)), convert: number(standard.filter((f) => f.fix === 'convert').length), twin: number(standard.filter((f) => f.fix === 'twin').length), rest: number(standard.filter((f) => !f.fix).length) })}</p>{/if}
				<DataTable columns={standardColumns} rows={standard} rowKey={(r) => r.path} selectable bind:selected={standardPicked} labels={tableLabels()}>
					{#snippet bulk()}
						<!-- DRAFT: not wired yet – converts the chosen files to new WebPs beside them and relinks every use -->
						<Button size="sm" disabled data-testid="assets-standard-convert">{m['assets.standard_convert']({ files: number(standardChosen.length) })}</Button>
					{/snippet}
					{#snippet cell(r, column)}
						{#if column.id === 'path'}<a href={href({ asset: r.asset })} title={r.path}>{r.path.split('/').pop()}</a><div class="sc-caption">{r.path.split('/').slice(0, -1).join('/')}</div>
						{:else if column.id === 'advice'}<Badge tone="warning">{r.advice}</Badge>
						{:else if column.id === 'fix'}
							{#if r.fix}<span title={r.target}>→ {r.target?.split('/').pop()}</span><div class="sc-caption">{r.fix === 'twin' ? m['assets.fix_twin']() : m['assets.fix_new']()}</div>
							{:else if r.kind === 'video'}<span class="sc-caption">{m['assets.fix_video']()}</span>
							{:else}<span class="sc-caption">{m['assets.fix_source']()}</span>{/if}
						{:else if column.id === 'usages'}{number(r.usages)}
						{:else}{bytes(r.size)}{/if}
					{/snippet}
				</DataTable>
			{/if}
		{:else}
			{#if !storage}
				<Skeleton variant="table" columns={4} label={m['shell.loading']()} />
			{:else}
				<Section title={m['assets.storage_title']()}>
					<dl class="facts">
						<dt>{m['assets.storage_primary']()}</dt><dd data-testid="assets-primary">{storage.primary}</dd>
						<dt>{m['assets.storage_local']()}</dt><dd><code>{storage.local.where}</code></dd>
						<dt>bunny.net</dt><dd>{storage.bunny ? storage.bunny.where : m['assets.bunny_off']()}</dd>
						<dt>{m['assets.public_base']()}</dt><dd><code>{storage.public_base}</code></dd>
						<dt>{m['assets.totals']()}</dt><dd>{m['assets.totals_value']({ assets: number(storage.totals.assets), files: number(storage.totals.files), size: bytes(storage.totals.bytes) })}</dd>
					</dl>
					<ul class="kinds">
						{#each Object.entries(storage.totals.kinds) as [k, t] (k)}<li><Badge tone="neutral">{kindLabel(k)}</Badge> {number(t.assets)} · {number(t.files)} · {bytes(t.bytes)}</li>{/each}
					</ul>
				</Section>
				<Section title={m['assets.settings_title']()}>
					<p class="sc-caption">{m['assets.settings_hint']()}</p>
					<DataTable columns={settingColumns} rows={Object.entries(storage.settings).map(([name, s]) => ({ name, ...s }))} rowKey={(r) => r.name} labels={tableLabels()}>
						{#snippet cell(r, column)}
							{#if column.id === 'name'}<code>{r.name}</code>
							{:else if column.id === 'status'}<Badge tone={settingTone(r.status)}>{r.status}</Badge>
							{:else if column.id === 'value'}{r.value ?? '—'}
							{:else}<span class="sc-caption">{r.note || ''}</span>{/if}
						{/snippet}
					</DataTable>
				</Section>
				<Section title={m['assets.bandwidth_title']()}>
					{#if storage.bandwidth}
						<p data-testid="assets-bandwidth">{m['assets.bandwidth_value']({ days: storage.bandwidth.days, size: bytes(storage.bandwidth.bandwidth), requests: number(storage.bandwidth.requests), hit: Math.round(storage.bandwidth.cache_hit_rate) })}</p>
						{@const peak = Math.max(1, ...storage.bandwidth.daily.map((d) => d.bytes))}
						<ol class="bars" aria-label={m['assets.bandwidth_title']()}>
							{#each storage.bandwidth.daily as d (d.day)}<li title="{d.day}: {bytes(d.bytes)}"><span style:height="{Math.max(2, (d.bytes / peak) * 100)}%"></span></li>{/each}
						</ol>
					{:else}
						{#if storage.bandwidth_error}<p class="error-box">{storage.bandwidth_error}</p>{/if}
						<p class="sc-caption">{storage.bunny?.api_key ? m['assets.bandwidth_load_hint']() : m['assets.bandwidth_no_key']()}</p>
						{#if storage.bunny?.api_key}<Button size="sm" variant="secondary" disabled={busy === 'bandwidth'} onclick={loadBandwidth}>{m['assets.bandwidth_load']()}</Button>{/if}
					{/if}
				</Section>
			{/if}
		{/if}
	{/snippet}
</Tabs>

<Sheet open={detailOpen} side="right" widthKey="asset" title={m['assets.detail_title']()} closeLabel={m['common.close']()} resizeLabel={m['shell.resize_panel']()} onclose={() => nav({ asset: null })}>
	{#if query.filters.asset}
		{#key query.filters.asset}
			<AssetDetail id={query.filters.asset} {cdn} onchange={() => reload++} ondeleted={() => (nav({ asset: null }), reload++)} />
		{/key}
	{/if}
</Sheet>

<style>
	.grid { /* grid-ok: the asset cards, not fields */
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(11rem, 1fr));
		gap: var(--space-1);
	}
	.card {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		padding: var(--space-05);
		border: var(--border-width-regular) solid var(--border-muted);
		color: var(--ink);
		text-decoration: none;
		min-width: 0;
		height: 100%;
		box-sizing: border-box;
	}
	.card:hover {
		border-color: var(--ink);
	}
	.img {
		aspect-ratio: 1;
		border: var(--border-width-hairline) solid var(--ink);
		display: flex;
		align-items: center;
		justify-content: center;
		overflow: hidden;
	}
	.img img {
		max-width: 100%;
		max-height: 100%;
		object-fit: contain;
	}
	.name,
	.line {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.badges {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-025);
	}
	.ref {
		display: inline-flex;
		gap: var(--space-025);
		align-items: center;
		margin-right: var(--space-05);
	}
	.notice {
		border-left: 0.5rem solid var(--primary-cyan);
		padding-left: var(--space-05);
	}
	.facts {
		display: grid;
		grid-template-columns: max-content 1fr;
		gap: var(--space-05) var(--space-1);
		margin: 0;
	}
	.facts dd {
		margin: 0;
		overflow-wrap: anywhere;
	}
	.kinds {
		list-style: none;
		padding: 0;
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-1);
	}
	.bars {
		list-style: none;
		padding: 0;
		margin: var(--space-1) 0 0;
		display: flex;
		align-items: flex-end;
		gap: 2px;
		height: 6rem;
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	.bars li {
		flex: 1;
		height: 100%;
		display: flex;
		align-items: flex-end;
	}
	.bars span {
		display: block;
		width: 100%;
		background: var(--ink);
	}
</style>
