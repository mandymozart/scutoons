<script lang="ts">
	import { infoBar } from '$lib/info.ts';
	import PageHelp from '$lib/components/page-help.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { ActionBar, Button, DataTable, SearchField, Skeleton, Segmented, type Column } from '@scutoons/ui';
	import type { AdminServerError, AdminServerErrors } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { errorText, number } from '$lib/format.ts';
	import { tableLabels } from '$lib/labels.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PageHeader from '$lib/components/page-header.svelte';

	/**
	 * Server errors (BACKLOG 10.11, owner 2026-10-02): the newest entries of the API's and the qr app's PHP error log,
	 * newest first – messages only (the API masks e-mail addresses, tokens, passwords). Filters: the log, a day, text.
	 */
	const source = $derived(page.url.searchParams.get('source') === 'qr' ? 'qr' : 'api');
	const day = $derived(page.url.searchParams.get('day') ?? '');
	const q = $derived(page.url.searchParams.get('q') ?? '');

	let result = $state<AdminServerErrors | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(false);
	let reloads = $state(0);

	$effect(() => {
		void reloads;
		const params = { source, day, q } as const;
		loading = true;
		error = null;
		admin
			.serverErrors(params)
			.then((r) => (result = r))
			.catch((e) => (error = errorText(e)))
			.finally(() => (loading = false));
	});

	const nav = (patch: Record<string, string>) => {
		const url = new URL(page.url);
		for (const [k, v] of Object.entries(patch)) v ? url.searchParams.set(k, v) : url.searchParams.delete(k);
		void goto(url, { keepFocus: true, noScroll: true });
	};
	const columns = $derived<Column[]>([
		{ id: 'at', label: m['server_errors.col_when']() },
		{ id: 'level', label: m['server_errors.col_level']() },
		{ id: 'message', label: m['server_errors.col_message']() }
	]);
	const LEVELS = { fatal: () => m['server_errors.level_fatal'](), error: () => m['server_errors.level_error'](), warning: () => m['server_errors.level_warning'](), notice: () => m['server_errors.level_notice']() };
	const size = (bytes: number) => (bytes >= 1048576 ? `${(bytes / 1048576).toFixed(1)} MB` : `${Math.ceil(bytes / 1024)} KB`);
	const rows = $derived((result?.entries ?? []).map((e, i) => ({ ...e, key: `${e.at}-${i}` })));
</script>

<PageHeader type={m['nav.group_system']()} title={m['server_errors.title']()} meta={result?.exists ? m['server_errors.count']({ count: number(result.entries.length) }) : undefined} />

{#snippet barStart()}
	<Segmented
		label={m['server_errors.source']()}
		value={source}
		options={[
			{ value: 'api', label: m['server_errors.source_api']() },
			{ value: 'qr', label: m['server_errors.source_qr']() }
		]}
		onchange={(v) => nav({ source: v === 'qr' ? 'qr' : '', day: '' })}
	/>
	<label class="bar-filter">
		<span>{m['server_errors.day']()}</span>
		<select value={day} onchange={(e) => nav({ day: e.currentTarget.value })} data-testid="server-errors-day">
			<option value="">{m['server_errors.all_days']()}</option>
			{#each result?.days ?? [] as d (d)}<option value={d}>{d}</option>{/each}
		</select>
	</label>
	<SearchField compact label={m['server_errors.search']()} placeholder={m['server_errors.search']()} value={q} onsearch={(text) => nav({ q: text })} />
	<Button size="sm" variant="secondary" onclick={() => reloads++} disabled={loading}>{m['server_errors.reload']()}</Button>
{/snippet}

<ActionBar start={barStart} label={m['shell.page_actions']()} moreLabel={m['shell.more']()} {...infoBar()} infoTitle={m['server_errors.title']()}>
	{#snippet info()}<PageHelp page="server-errors" />{/snippet}
</ActionBar>

{#if error}<p class="error-box" role="alert">{error}</p>{/if}

{#if !result}
	<Skeleton variant="table" columns={3} label={m['shell.loading']()} />
{:else if !result.exists}
	<p class="sc-caption" data-testid="server-errors-none">{m['server_errors.no_log']()}</p>
{:else}
	{#if result.truncated}<p class="sc-caption">{m['server_errors.truncated']({ size: size(result.size) })}</p>{/if}
	{#if !rows.length}
		<p class="sc-caption" data-testid="server-errors-none">{m['server_errors.none']()}</p>
	{:else}
		<DataTable {columns} {rows} rowKey={(e) => e.key} {loading} labels={tableLabels()}>
			{#snippet cell(e: AdminServerError & { key: string }, column)}
				{#if column.id === 'at'}<span class="when">{e.at?.replace('T', ' ') ?? '—'}</span>
				{:else if column.id === 'level'}<span class="level level-{e.level}">{LEVELS[e.level]()}</span>
				{:else}
					<div class="message">{e.message}</div>
					{#if e.detail}
						<details>
							<summary>{m['server_errors.detail']()}</summary>
							<pre>{e.detail}</pre>
						</details>
					{/if}
				{/if}
			{/snippet}
		</DataTable>
	{/if}
{/if}

<style>
	.when {
		white-space: nowrap;
		font-variant-numeric: tabular-nums;
	}
	.level {
		white-space: nowrap;
	}
	.level-fatal,
	.level-error {
		color: var(--state-error);
	}
	.level-warning {
		color: var(--state-warning);
	}
	.message {
		overflow-wrap: anywhere;
		white-space: pre-wrap;
	}
	pre {
		margin: var(--space-05) 0 0;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
		font-size: var(--type-caption-size);
		color: var(--text-muted);
	}
</style>
