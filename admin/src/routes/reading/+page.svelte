<script lang="ts">
	import { infoBar } from '$lib/info.ts';
	import PageHelp from '$lib/components/page-help.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { ActionBar, Button, DataTable, Skeleton, type Column } from '@scutoons/ui';
	import type { AdminReadingStats } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { errorText, minute, number, percent } from '$lib/format.ts';
	import { tableLabels } from '$lib/labels.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PageHeader from '$lib/components/page-header.svelte';
	import ReadingStops from '$lib/components/reading-stops.svelte';
	import { chapterLabel, toonName } from '$lib/reading.ts';

	/**
	 * Reading (BACKLOG 10.69, owner 2026-10-11: "wie weit gelesen wurde in einem Chapter – das ist ja auch sehr
	 * aussagekräftig, wo eine Person ist"): per chapter, over every member's save game – started, finished, and where
	 * those not finished yet stand. Guests' save games are only in their browsers. Filter: one toon (in the URL).
	 */
	type Row = AdminReadingStats['chapters'][number];
	const toon = $derived(page.url.searchParams.get('toon') ?? '');

	let result = $state<AdminReadingStats | null>(null);
	let toons = $state<{ id: string; name: string }[]>([]);
	let error = $state<string | null>(null);
	let loading = $state(false);
	let reloads = $state(0);

	$effect(() => {
		admin
			.catalog()
			.then((c) => (toons = c.toons.map((t) => ({ id: t.id, name: t.name }))))
			.catch(() => {});
	});
	$effect(() => {
		void reloads;
		const current = toon;
		loading = true;
		error = null;
		admin
			.reading(current || undefined)
			.then((r) => (result = r))
			.catch((e) => (error = errorText(e)))
			.finally(() => (loading = false));
	});

	const setToon = (value: string) => {
		const url = new URL(page.url);
		if (value) url.searchParams.set('toon', value);
		else url.searchParams.delete('toon');
		void goto(url, { keepFocus: true, noScroll: true });
	};
	const columns = $derived<Column[]>([
		{ id: 'chapter', label: m['reading.col_chapter']() },
		{ id: 'started', label: m['reading.col_started']() },
		{ id: 'finished', label: m['reading.col_finished']() },
		{ id: 'reading', label: m['reading.col_reading']() },
		{ id: 'stops', label: m['reading.col_stops']() },
		{ id: 'last_read_at', label: m['reading.col_last']() }
	]);
	const stopsLabel = (stops: number[]) => m['reading.stops_label']({ stops: stops.map((n, i) => `${i * 10}–${(i + 1) * 10} %: ${n}`).join(', ') });
</script>

<PageHeader type={m['nav.group_people']()} title={m['reading.title']()} meta={result ? m['reading.count']({ count: number(result.readers) }) : undefined} />

{#snippet barStart()}
	<label class="bar-filter">
		<span>{m['reading.toon']()}</span>
		<select value={toon} onchange={(e) => setToon(e.currentTarget.value)} data-testid="reading-toon">
			<option value="">{m['reading.all_toons']()}</option>
			{#each toons as t (t.id)}<option value={t.id}>{t.name}</option>{/each}
		</select>
	</label>
	<Button size="sm" variant="secondary" onclick={() => reloads++} disabled={loading}>{m['reading.reload']()}</Button>
{/snippet}

<ActionBar start={barStart} label={m['shell.page_actions']()} moreLabel={m['shell.more']()} {...infoBar()} infoTitle={m['reading.title']()}>
	{#snippet info()}<PageHelp page="reading" />{/snippet}
</ActionBar>

{#if error}<p class="error-box" role="alert">{error}</p>{/if}

{#if !result}
	{#if !error}<Skeleton variant="table" columns={6} label={m['shell.loading']()} />{/if}
{:else if !result.chapters.length}
	<p class="sc-caption" data-testid="reading-none">{m['reading.none']()}</p>
{:else}
	<DataTable {columns} rows={result.chapters} rowKey={(c) => `${c.toon}/${c.chapter}`} {loading} labels={tableLabels()}>
		{#snippet cell(c: Row, column)}
			{#if column.id === 'chapter'}{chapterLabel(toonName(toons, c.toon), c.chapter)}
			{:else if column.id === 'started'}<span class="num">{number(c.started)}</span>
			{:else if column.id === 'finished'}<span class="num">{number(c.finished)} <span class="share">{percent(c.finished, c.started)}</span></span>
			{:else if column.id === 'reading'}<span class="num">{number(c.reading)}</span>
			{:else if column.id === 'stops'}{#if c.reading}<ReadingStops stops={c.stops} label={stopsLabel(c.stops)} />{:else}—{/if}
			{:else}{minute(c.last_read_at)}{/if}
		{/snippet}
	</DataTable>
{/if}

<style>
	.num {
		white-space: nowrap;
		font-variant-numeric: tabular-nums;
	}
	.share {
		color: var(--text-muted);
	}
</style>
