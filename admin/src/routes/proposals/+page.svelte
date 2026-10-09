<script lang="ts">
	// Proposals (agents/MCP.md "Draft → link", owner 2026-10-09): changes an admin's own AI proposed through the SCUTOONS
	// MCP (propose_changes). The list by status (open first), the proposal in the right dock (`?open=<id>`, the link the
	// AI hands out): before → after, Apply or Dismiss (ChangesCard). Nothing changes until an admin applies it here.
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { ActionBar, Badge, Segmented, Skeleton, SplitView, listHref } from '@scutoons/ui';
	import type { AdminProposal, AdminProposalRow, ProposalStatus } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { errorText, minute, number } from '$lib/format.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PageHeader from '$lib/components/page-header.svelte';
	import ChangesCard from '$lib/components/changes-card.svelte';

	const STATUSES: ProposalStatus[] = ['open', 'applied', 'dismissed'];
	const status = $derived((STATUSES as string[]).includes(page.url.searchParams.get('status') ?? '') ? (page.url.searchParams.get('status') as ProposalStatus) : 'open');
	const open = $derived(page.url.searchParams.get('open'));

	let items = $state<AdminProposalRow[] | null>(null);
	let error = $state<string | null>(null);
	let reload = $state(0);
	let current = $state<AdminProposal | null>(null);
	let detailError = $state<string | null>(null);

	$effect(() => {
		const s = status;
		void reload;
		error = null;
		admin
			.proposals(s)
			.then((r) => (items = r.items))
			.catch((e) => (error = errorText(e)));
	});
	$effect(() => {
		const id = open;
		current = null;
		detailError = null;
		if (!id) return;
		admin
			.proposal(id)
			.then((r) => (current = r.proposal))
			.catch((e) => (detailError = errorText(e)));
	});

	const nav = (patch: Record<string, string | number | null>) => goto(listHref(page.url, patch), { keepFocus: true, noScroll: true });
	const statusLabel = (s: ProposalStatus) => m[`proposals.status_${s}`]();
	const tone = (s: ProposalStatus) => (s === 'open' ? 'draft' : s === 'applied' ? 'published' : 'neutral');

	function decided(p: AdminProposal) {
		current = p;
		reload++;
	}
</script>

<PageHeader type={m['proposals.type']()} title={m['proposals.title']()} meta={items ? m['proposals.count']({ count: number(items.length) }) : undefined} />

{#snippet barStart()}
	<Segmented label={m['proposals.status']()} value={status} options={STATUSES.map((s) => ({ value: s, label: statusLabel(s) }))} onchange={(v) => nav({ status: v === 'open' ? null : v, open: null })} />
{/snippet}

<SplitView
	open={!!open}
	title={current?.title ?? ''}
	closeLabel={m['common.close']()}
	foldLabel={m['shell.fold_panel']()}
	maximizeLabel={m['shell.maximize_panel']()}
	restoreLabel={m['shell.restore_panel']()}
	onclose={() => nav({ open: null })}
>
	{#snippet list()}
		{#if error}
			<p class="sc-error">{error}</p>
		{:else if !items}
			<Skeleton variant="table" columns={3} label={m['shell.loading']()} />
		{:else}
			<ul class="proposals" data-testid="proposals">
				{#each items as p (p.id)}
					<li class:is-open={p.id === open}>
						<a href={listHref(page.url, { open: p.id })} data-testid="proposal-{p.id}">
							<span class="what">{p.title}<span class="meta">{m['proposals.changes']({ count: p.count })} · {p.author} · {minute(p.created_at)}</span></span>
							<Badge tone={tone(p.status)}>{statusLabel(p.status)}</Badge>
						</a>
					</li>
				{/each}
			</ul>
		{/if}
	{/snippet}
	{#snippet detail()}
		{#if detailError}
			<p class="sc-error">{detailError}</p>
		{:else if !current}
			<Skeleton label={m['shell.loading']()} />
		{:else}
			{#key current.id + current.status}
				<ChangesCard proposal={current} ondecided={decided} />
			{/key}
		{/if}
	{/snippet}
</SplitView>

<ActionBar start={barStart} label={m['shell.page_actions']()} moreLabel={m['shell.more']()} />

<style>
	.proposals {
		display: flex;
		flex-direction: column;
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.proposals li {
		border-bottom: var(--border-width-hairline) solid var(--border-subtle);
	}
	.proposals li:first-child {
		border-top: var(--border-width-hairline) solid var(--border-subtle);
	}
	.proposals a {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-1);
		min-height: var(--control-height);
		padding: var(--space-05) var(--space-05);
		color: var(--ink);
		text-decoration: none;
	}
	.proposals a:hover {
		background: var(--surface-hover);
	}
	.proposals li.is-open a {
		background: var(--select-live-bg);
		box-shadow: inset var(--border-width-regular) 0 0 var(--select-live-border);
	}
	.what {
		display: flex;
		flex-direction: column;
		min-width: 0;
		overflow-wrap: anywhere;
	}
	.meta {
		font-size: var(--type-caption-size);
		color: var(--text-muted);
	}
</style>
