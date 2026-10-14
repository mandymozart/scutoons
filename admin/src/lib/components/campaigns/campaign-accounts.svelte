<script lang="ts">
	import { Button, SearchField, Segmented, Skeleton, confirmDialog } from '@scutoons/ui';
	import type { AccountFilterParams, AdminSegment, Campaign, CampaignAccounts, CampaignInviters } from '@scutoons/api';
	import AccountFilters from '$lib/components/account-filters.svelte';
	import { admin } from '$lib/api.ts';
	import { campaignError } from '$lib/campaigns.ts';
	import { day, number } from '$lib/format.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * Who may invite (owner 2026-10-14: "a checkbox on a list of accounts, which also has a all checkbox … also if
	 * paginated"; segments): every account (also later ones), the ticked accounts, or a segment's members. The list
	 * takes the account filters; "Tick all N" adds every account they find (every page).
	 */
	let { campaign }: { campaign: Campaign } = $props();

	const toaster = getToaster();
	const PAGE_SIZE = 25;
	let data = $state<CampaignAccounts | null>(null);
	let segments = $state<AdminSegment[]>([]);
	let q = $state('');
	let filters = $state<Record<string, string>>({});
	let pageNo = $state(1);
	let busy = $state(false);

	admin
		.segments()
		.then((r) => (segments = r.items))
		.catch(() => {});

	function load() {
		admin
			.campaignAccounts(campaign.id, { q: q || undefined, ...(filters as AccountFilterParams), page: pageNo, page_size: PAGE_SIZE })
			.then((r) => (data = r))
			.catch((e) => toaster.addToast({ message: campaignError(e), type: 'error' }));
	}
	$effect(() => {
		void q;
		void filters;
		void pageNo;
		load();
	});

	async function set(body: Parameters<typeof admin.setCampaignAccounts>[1]) {
		busy = true;
		try {
			await admin.setCampaignAccounts(campaign.id, body);
			load();
		} catch (e) {
			toaster.addToast({ message: campaignError(e), type: 'error' });
		} finally {
			busy = false;
		}
	}

	function setMode(mode: CampaignInviters) {
		if (mode === 'all') void set({ all: true });
		else if (mode === 'chosen') void set({ all: false });
		else if (segments[0]) void set({ segment_id: data?.segment_id ?? segments[0].id });
	}

	const filtered = $derived(!!q || Object.values(filters).some((v) => v));
	async function tickMatching() {
		if (!data) return;
		const ok = await confirmDialog({ title: m['campaigns.tick_matching_title']({ count: number(data.total) }), message: m['campaigns.tick_matching_message'](), confirmLabel: m['campaigns.tick_matching']() });
		if (ok) await set({ add_matching: true, q: q || undefined, filters: filters as AccountFilterParams });
	}

	const pages = $derived(data ? Math.max(1, Math.ceil(data.total / PAGE_SIZE)) : 1);
</script>

<div class="accounts">
	<Segmented
		label={m['campaigns.inviters']()}
		options={[
			{ value: 'all', label: m['campaigns.inviters_all']() },
			{ value: 'chosen', label: m['campaigns.inviters_chosen']() },
			...(segments.length ? [{ value: 'segment' as const, label: m['campaigns.inviters_segment']() }] : [])
		]}
		value={data?.mode ?? campaign.inviters}
		onchange={(v) => setMode(v as CampaignInviters)}
	/>
	{#if data}
		<p class="muted">
			{data.mode === 'all' ? m['campaigns.inviters_all_hint']() : data.mode === 'segment' ? m['campaigns.inviters_segment_hint']() : m['campaigns.inviters_chosen_hint']()}
			{m['campaigns.inviters_count']({ count: number(data.chosen) })}
		</p>
	{/if}
	{#if data?.mode === 'segment'}
		<label class="bar-filter">
			<span>{m['accounts.segment']()}</span>
			<select value={data.segment_id ?? ''} onchange={(e) => set({ segment_id: e.currentTarget.value })}>
				{#each segments as s (s.id)}<option value={s.id}>{s.name} ({number(s.members)})</option>{/each}
			</select>
		</label>
	{/if}

	<div class="filters">
		<SearchField compact label={m['users.search']()} placeholder={m['users.search_placeholder']()} value={q} onsearch={(v) => ((q = v), (pageNo = 1))} />
		<AccountFilters {filters} onchange={(patch) => ((filters = { ...filters, ...patch }), (pageNo = 1))} />
	</div>

	{#if filtered && data && data.mode === 'chosen' && data.total > 0}
		<div class="matching">
			<span>{m['segments.matching_line']({ count: number(data.total) })}</span>
			<Button size="sm" variant="secondary" disabled={busy} onclick={tickMatching}>{m['campaigns.tick_matching']()}</Button>
		</div>
	{/if}

	{#if !data}
		<Skeleton variant="table" columns={3} label={m['shell.loading']()} />
	{:else}
		<table class="list">
			<thead>
				<tr>
					<th class="cb">
						<input
							type="checkbox"
							aria-label={m['campaigns.tick_all']()}
							title={m['campaigns.tick_all']()}
							checked={data.mode === 'all'}
							disabled={busy || data.mode === 'segment'}
							onchange={(e) => set({ all: e.currentTarget.checked })}
						/>
					</th>
					<th>{m['users.col_email']()}</th>
					<th>{m['users.col_created']()}</th>
				</tr>
			</thead>
			<tbody>
				{#each data.items as a (a.id)}
					<tr class:chosen={a.chosen}>
						<td class="cb">
							<input
								type="checkbox"
								aria-label={a.email}
								checked={a.chosen}
								disabled={busy || data.mode === 'segment'}
								onchange={(e) => set(e.currentTarget.checked ? { add: [a.id] } : { remove: [a.id] })}
							/>
						</td>
						<td><a href="/users/{encodeURIComponent(a.id)}">{a.email}</a></td>
						<td>{day(a.created_at)}</td>
					</tr>
				{/each}
			</tbody>
		</table>
		{#if pages > 1}
			<div class="pages">
				<Button size="sm" variant="secondary" disabled={pageNo <= 1} onclick={() => pageNo--}>{m['campaigns.page_prev']()}</Button>
				<span class="muted">{m['campaigns.page_of']({ page: number(pageNo), pages: number(pages) })}</span>
				<Button size="sm" variant="secondary" disabled={pageNo >= pages} onclick={() => pageNo++}>{m['campaigns.page_next']()}</Button>
			</div>
		{/if}
	{/if}
</div>

<style>
	/* one column that may shrink: a wide table scrolls inside it instead of widening the page */
	.accounts {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		gap: var(--space-1);
	}
	.accounts > p {
		margin: 0;
	}
	.filters,
	.matching,
	.pages {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-1);
	}
	.list {
		width: 100%;
		border-collapse: collapse;
		border: var(--border-width-regular) solid var(--ink);
	}
	.list th {
		text-align: left;
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--text-muted);
		padding: var(--space-05);
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	.list td {
		padding: var(--space-05);
		border-bottom: var(--border-width-hairline) solid var(--border-muted);
	}
	.list .cb {
		width: 2.5rem;
	}
	.list tr.chosen td {
		background: var(--surface-hover);
	}
</style>
