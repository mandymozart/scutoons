<script lang="ts">
	import { infoBar } from '$lib/info.ts';
	import PageHelp from '$lib/components/page-help.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { ActionBar, Badge, Button, DataTable, RecordInspector, Section, Skeleton, Tabs, ToonAvatar, type Column } from '@scutoons/ui';
	import type { AdminCatalog } from '@scutoons/api';
	import { loadCatalog } from '$lib/catalog.ts';
	import { toonAvatarUrl } from '$lib/config.ts';
	import { errorText } from '$lib/format.ts';
	import { inspectorLabels, tableLabels } from '$lib/labels.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import PageHeader from '$lib/components/page-header.svelte';
	import RespawnEditor from '$lib/components/respawn-editor.svelte';

	/** The server's catalog config: items (+ recipes), editions, toons, the permissions map – read-only except the items' respawn (RespawnEditor). */
	let catalog = $state<AdminCatalog | null>(null);
	let error = $state<string | null>(null);

	$effect(() => {
		loadCatalog()
			.then((c) => (catalog = c))
			.catch((e) => (error = errorText(e)));
	});

	const tabs = $derived([
		{ id: 'items', label: m['catalog.items']() },
		{ id: 'editions', label: m['catalog.editions']() },
		{ id: 'toons', label: m['catalog.toons']() },
		{ id: 'permissions', label: m['catalog.permissions']() }
	]);
	const tab = $derived(page.url.searchParams.get('tab') ?? 'items');

	const text = (value: unknown) => {
		if (value && typeof value === 'object') {
			const localized = value as Record<string, string>;
			return localized[getLocale()] ?? localized.en ?? Object.values(localized)[0] ?? '';
		}
		return value === null || value === undefined || value === '' ? '—' : String(value);
	};

	type Row = Record<string, any>;
	const itemColumns = $derived<Column[]>([
		{ id: 'id', label: 'id' },
		{ id: 'name', label: m['catalog.name']() },
		{ id: 'toon', label: m['events.col_toon'](), secondary: true },
		{ id: 'kind', label: m['catalog.kind']() },
		{ id: 'category', label: m['catalog.category'](), secondary: true },
		{ id: 'slot', label: m['catalog.slot'](), secondary: true },
		{ id: 'edition_id', label: m['common.edition'](), secondary: true },
		{ id: 'flags', label: m['catalog.flags']() }
	]);
	const editionColumns = $derived<Column[]>([
		{ id: 'id', label: 'id' },
		{ id: 'name', label: m['catalog.name']() },
		{ id: 'toon', label: m['events.col_toon']() },
		{ id: 'edition_count', label: m['catalog.edition_count'](), align: 'end' },
		{ id: 'excluded_number', label: m['batches.excluded'](), align: 'end', secondary: true },
		{ id: 'variants', label: m['catalog.variants'](), secondary: true },
		{ id: 'links', label: '' }
	]);
	const typeRows = $derived(
		catalog ? Object.entries((catalog.permissions.types ?? {}) as Record<string, Row>).map(([type, rule]): Row => ({ type, ...rule })) : []
	);
</script>

<PageHeader type={m['nav.group_data']()} title={m['catalog.title']()} meta={m['catalog.meta']()} />

<ActionBar label={m['shell.page_actions']()} moreLabel={m['shell.more']()} {...infoBar()} infoTitle={m['catalog.title']()}>
	{#snippet info()}<PageHelp page="catalog" />{/snippet}
	{#snippet start()}
		{#if catalog}
			<span class="muted">{m['catalog.counts']({ items: catalog.catalog.items.length, editions: catalog.editions.length, toons: catalog.toons.length })}</span>
		{/if}
	{/snippet}
	{#snippet end()}
		<Button size="sm" variant="secondary" href="/batches">{m['nav.batches']()}</Button>
	{/snippet}
</ActionBar>

{#if error}
	<p class="error-box" role="alert">{error}</p>
{:else if !catalog}
	<Skeleton variant="table" label={m['shell.loading']()} />
{:else}
	<Tabs {tabs} value={tab} label={m['catalog.title']()} onchange={(id) => goto(`?tab=${id}`, { keepFocus: true, noScroll: true, replaceState: true })}>
		{#snippet children(active)}
			{#if active === 'items'}
				<DataTable columns={itemColumns} rows={catalog!.catalog.items as Row[]} rowKey={(i) => i.id} labels={tableLabels()}>
					{#snippet cell(item, column)}
						{#if column.id === 'name'}{text(item.name)}
						{:else if column.id === 'flags'}
							<span class="badges">
								{#if item.starter}<Badge>{m['catalog.starter']()}</Badge>{/if}
								{#if item.stackable}<Badge>{m['catalog.stackable']()}</Badge>{/if}
								{#each item.skills ?? [] as skill, i (i)}<Badge tone="accent">{skill.type}</Badge>{/each}
								{#if item.format}<Badge>{item.format} · {item.album}</Badge>{/if}
								{#each item.plays ?? [] as format (format)}<Badge tone="accent">plays {format}</Badge>{/each}
							</span>
						{:else}{text(item[column.id])}{/if}
					{/snippet}
				</DataTable>
				<!-- when picked-up placed items reappear (an update of the item in the catalog tables) -->
				<Section title={m['catalog.respawn']()}>
					<RespawnEditor names={Object.fromEntries(catalog!.catalog.items.map((i: Row) => [i.id, text(i.name)]))} />
				</Section>
				{#if catalog!.catalog.recipes?.length}
					<Section title={m['catalog.recipes']()}>
						<ul class="recipes">
							{#each catalog!.catalog.recipes as recipe (recipe.id)}
								<li>
									{(recipe.inputs ?? []).map((i: Row) => `${i.qty} × ${i.item}`).join(' + ')} → {recipe.output?.qty} × {recipe.output?.item}
								</li>
							{/each}
						</ul>
					</Section>
				{/if}
				<!-- albums (agents/ITEMS.md "Players, albums, related items"): the music media items carry, and who may
				     download / listen (access.download default members, access.listen default owners) – the catalog table
				     catalog_albums (agents/CATALOG.md) -->
				{#if catalog!.catalog.albums?.length}
					<Section title={m['catalog.albums']()}>
						<DataTable
							columns={[
								{ id: 'id', label: 'id' },
								{ id: 'title', label: m['catalog.name']() },
								{ id: 'tracks', label: m['catalog.album_tracks'](), align: 'end' },
								{ id: 'carriers', label: m['catalog.album_carriers']() },
								{ id: 'download', label: m['catalog.album_download'](), secondary: true },
								{ id: 'access', label: m['catalog.album_access']() }
							]}
							rows={catalog!.catalog.albums as Row[]}
							rowKey={(a) => a.id}
							labels={tableLabels()}
						>
							{#snippet cell(a, column)}
								{#if column.id === 'title'}{text(a.title)}
								{:else if column.id === 'tracks'}{(a.tracks ?? []).length}
								{:else if column.id === 'carriers'}{catalog!.catalog.items.filter((i: Row) => i.album === a.id).map((i: Row) => `${i.id} (${i.format})`).join(', ') || '—'}
								{:else if column.id === 'access'}
									<span class="badges">
										<Badge>download: {a.access?.download ?? 'members'}</Badge>
										<Badge>listen: {a.access?.listen ?? 'owners'}</Badge>
									</span>
								{:else}{text(a[column.id])}{/if}
							{/snippet}
						</DataTable>
					</Section>
				{/if}
			{:else if active === 'editions'}
				<DataTable columns={editionColumns} rows={catalog!.editions as Row[]} rowKey={(e) => e.id} labels={tableLabels()}>
					{#snippet cell(e, column)}
						{#if column.id === 'variants'}{(e.variants ?? []).map((v: Row) => v.name).join(', ') || '—'}
						{:else if column.id === 'links'}
							<span class="badges">
								<a href="/codes?edition_id={encodeURIComponent(e.id)}">{m['nav.codes']()}</a>
								<a href="/batches?edition_id={encodeURIComponent(e.id)}">{m['nav.batches']()}</a>
							</span>
						{:else}{text(e[column.id])}{/if}
					{/snippet}
				</DataTable>
			{:else if active === 'toons'}
				<DataTable
					columns={[
						{ id: 'avatar', label: '', width: '3.5rem' },
						{ id: 'id', label: 'id' },
						{ id: 'name', label: m['catalog.name']() },
						{ id: 'redirectUrl', label: m['codes.redirect']() }
					]}
					rows={catalog!.toons as Row[]}
					rowKey={(t) => t.id}
					labels={tableLabels()}
				>
					{#snippet cell(t, column)}
						{#if column.id === 'avatar'}<ToonAvatar src={toonAvatarUrl(t.id)} name={t.name} />
						{:else}{text(t[column.id])}{/if}
					{/snippet}
				</DataTable>
			{:else}
				<Section title={m['catalog.types']()}>
					<DataTable
						columns={[
							{ id: 'type', label: m['codes.col_type']() },
							{ id: 'claimable', label: m['catalog.claimable']() },
							{ id: 'signup', label: m['catalog.signup']() },
							{ id: 'grants', label: m['catalog.grants']() }
						]}
						rows={typeRows}
						rowKey={(r) => r.type}
						labels={tableLabels()}
					>
						{#snippet cell(r, column)}
							{#if column.id === 'claimable' || column.id === 'signup'}{r[column.id] ? m['common.yes']() : m['common.no']()}
							{:else if column.id === 'grants'}{(r.grants ?? []).join(', ') || '—'}
							{:else}{r.type}{/if}
						{/snippet}
					</DataTable>
				</Section>
				<Section title={m['catalog.raw_map']()}>
					<RecordInspector record={catalog!.permissions} labels={inspectorLabels()} />
				</Section>
			{/if}
		{/snippet}
	</Tabs>
{/if}

<style>
	.badges {
		display: inline-flex;
		flex-wrap: wrap;
		gap: var(--space-025) var(--space-05);
	}
	.recipes {
		margin: 0;
		padding-left: 1.2em;
	}
</style>
