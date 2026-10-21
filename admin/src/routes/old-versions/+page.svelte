<script lang="ts">
	import { infoBar } from '$lib/info.ts';
	import PageHelp from '$lib/components/page-help.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { ActionBar, Button, DataTable, Skeleton, confirmDialog, type Column } from '@scutoons/ui';
	import type { AdminOldVersions, AdminOldVersionsItem, AdminOldVersionsPurge } from '@scutoons/api';
	import { ApiError, admin } from '$lib/api.ts';
	import { bytes, errorText, number } from '$lib/format.ts';
	import { tableLabels } from '$lib/labels.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PageHeader from '$lib/components/page-header.svelte';

	/**
	 * Old versions (issue 154, owner 2026-10-21: "a purge old versions from a date backwards"): the dry run of every
	 * document version saved before a day (never the published, the newest or a review's), then one confirmed delete
	 * with the dry run's counts – the server refuses when they changed. The files only those versions named go with the
	 * next Assets → Unused cleanup.
	 */
	const today = new Date().toISOString().slice(0, 10);
	/** default: 30 days ago (UTC) */
	const before = $derived(page.url.searchParams.get('before') || new Date(Date.now() - 30 * 86400000).toISOString().slice(0, 10));

	let plan = $state<AdminOldVersions | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(false);
	let purging = $state(false);
	let reloads = $state(0);
	/** this visit's purges, newest first */
	let log = $state<AdminOldVersionsPurge[]>([]);

	$effect(() => {
		void reloads;
		const day = before;
		loading = true;
		error = null;
		admin
			.oldVersions(day)
			.then((r) => (plan = r))
			.catch((e) => (error = errorText(e)))
			.finally(() => (loading = false));
	});

	const setBefore = (day: string) => {
		const url = new URL(page.url);
		day ? url.searchParams.set('before', day) : url.searchParams.delete('before');
		void goto(url, { keepFocus: true, noScroll: true });
	};

	async function purge() {
		if (!plan?.versions) return;
		const ok = await confirmDialog({
			title: m['old_versions.confirm_title']({ versions: number(plan.versions) }),
			message: m['old_versions.confirm_message']({ documents: number(plan.documents), before: plan.before }),
			confirmLabel: m['old_versions.confirm']()
		});
		if (!ok) return;
		purging = true;
		error = null;
		try {
			log = [await admin.purgeOldVersions({ before: plan.before, documents: plan.documents, versions: plan.versions }), ...log];
		} catch (e) {
			error = e instanceof ApiError && e.code === 'plan_changed' ? m['old_versions.changed']() : errorText(e);
		} finally {
			purging = false;
			reloads++;
		}
	}

	const columns = $derived<Column[]>([
		{ id: 'key', label: m['old_versions.col_document']() },
		{ id: 'remove', label: m['old_versions.col_remove'](), align: 'end' },
		{ id: 'keep', label: m['old_versions.col_keep'](), align: 'end' },
		{ id: 'bytes', label: m['old_versions.col_size'](), align: 'end' },
		{ id: 'span', label: m['old_versions.col_span'](), secondary: true }
	]);
</script>

<PageHeader
	type={m['nav.group_data']()}
	title={m['old_versions.title']()}
	meta={plan ? m['old_versions.meta']({ versions: number(plan.versions), documents: number(plan.documents), size: bytes(plan.bytes) }) : undefined}
/>

{#snippet barStart()}
	<label class="bar-filter">
		<span>{m['old_versions.before']()}</span>
		<input type="date" max={today} value={before} onchange={(e) => setBefore(e.currentTarget.value)} data-testid="old-versions-before" />
	</label>
	<Button size="sm" variant="secondary" onclick={() => reloads++} disabled={loading}>{m['old_versions.reload']()}</Button>
	<Button size="sm" onclick={purge} disabled={loading || purging || !plan?.versions} data-testid="old-versions-purge">
		{m['old_versions.purge']({ versions: number(plan?.versions ?? 0) })}
	</Button>
{/snippet}

<ActionBar start={barStart} label={m['shell.page_actions']()} moreLabel={m['shell.more']()} {...infoBar()} infoTitle={m['old_versions.title']()}>
	{#snippet info()}<PageHelp page="old-versions" />{/snippet}
</ActionBar>

{#if error}<p class="error-box" role="alert">{error}</p>{/if}

{#each log as entry (entry.cutoff + entry.deleted + entry.bytes)}
	<p class="done" role="status" data-testid="old-versions-done">
		{m['old_versions.done']({ deleted: number(entry.deleted), documents: number(entry.documents), size: bytes(entry.bytes), before: entry.before })}
		<a href="/assets?view=unused">{m['old_versions.next_unused']()}</a>
	</p>
{/each}

{#if !plan}
	<Skeleton variant="table" columns={5} label={m['shell.loading']()} />
{:else if !plan.items.length}
	<p class="sc-empty" data-testid="old-versions-none">{m['old_versions.none']({ before: plan.before })}</p>
{:else}
	<p class="sc-caption">{m['old_versions.rule']()}</p>
	<DataTable {columns} rows={plan.items} rowKey={(r) => r.key} {loading} labels={tableLabels()}>
		{#snippet cell(r: AdminOldVersionsItem, column)}
			{#if column.id === 'key'}<span class="key">{r.key}</span>
			{:else if column.id === 'remove'}<span class="nums">{number(r.remove)}</span>
			{:else if column.id === 'keep'}<span class="nums">{number(r.keep)}</span>
			{:else if column.id === 'bytes'}<span class="nums">{bytes(r.bytes)}</span>
			{:else}<span class="nums">{r.oldest.slice(0, 10)} – {r.newest.slice(0, 10)}</span>
			{/if}
		{/snippet}
	</DataTable>
{/if}

<style>
	.key {
		overflow-wrap: anywhere;
	}
	.nums {
		white-space: nowrap;
		font-variant-numeric: tabular-nums;
	}
	.done {
		margin: 0 0 var(--space-1);
	}
</style>
