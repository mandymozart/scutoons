<script lang="ts">
	import { DataTable, Skeleton, type Column } from '@scutoons/ui';
	import type { Campaign, CampaignStats, InviteStatus } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { campaignError, inviteStatus } from '$lib/campaigns.ts';
	import { minute, number } from '$lib/format.ts';
	import { tableLabels } from '$lib/labels.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A campaign's numbers (owner 2026-10-14: "who is using the campaign and who actually signed up eventually through
	 * their invites"): totals, the invites by outcome, per member; "joined later" = the address made an account
	 * afterwards without the invite.
	 */
	let { campaign }: { campaign: Campaign } = $props();

	let stats = $state<CampaignStats | null>(null);
	let error = $state<string | null>(null);
	$effect(() => {
		admin
			.campaignStats(campaign.id)
			.then((r) => (stats = r))
			.catch((e) => (error = campaignError(e)));
	});

	const OUTCOMES: InviteStatus[] = ['joined', 'waiting', 'payment', 'returned', 'recalled', 'unpaid'];
	const columns = $derived<Column[]>([
		{ id: 'email', label: m['campaigns.col_member']() },
		{ id: 'invites', label: m['campaigns.col_invites'](), align: 'end' },
		{ id: 'joined', label: m['campaigns.col_joined'](), align: 'end' },
		{ id: 'open', label: m['campaigns.col_open'](), align: 'end' },
		{ id: 'joined_later', label: m['campaigns.col_joined_later'](), align: 'end' },
		{ id: 'last_invite', label: m['campaigns.col_last'](), secondary: true }
	]);
</script>

{#if error}<p class="error-box" role="alert">{error}</p>{/if}
{#if !stats}
	{#if !error}<Skeleton lines={6} label={m['shell.loading']()} />{/if}
{:else}
	<div class="tiles">
		<div><span class="big">{number(stats.inviters)}</span><span>{m['campaigns.stat_inviters']()}</span></div>
		<div><span class="big">{number(stats.sent)}</span><span>{m['campaigns.stat_sent']()}</span></div>
		<div><span class="big">{number(stats.by_status.joined)}</span><span>{m['campaigns.stat_joined']()}</span></div>
		<div><span class="big">{number(stats.by_status.waiting + stats.by_status.payment)}</span><span>{m['campaigns.stat_open']()}</span></div>
		<div><span class="big">{number(stats.joined_later)}</span><span>{m['campaigns.stat_joined_later']()}</span></div>
	</div>
	{#if stats.sent > 0}
		<div class="bars" aria-label={m['campaigns.stat_outcomes']()}>
			{#each OUTCOMES as o (o)}
				<div class="bar">
					<span>{inviteStatus(o)}</span>
					<span class="track"><span class="fill" class:joined={o === 'joined'} style:width="{(stats.by_status[o] / stats.sent) * 100}%"></span></span>
					<span class="nums">{number(stats.by_status[o])}</span>
				</div>
			{/each}
		</div>
		<DataTable {columns} rows={stats.members} rowKey={(r) => r.id} labels={tableLabels()}>
			{#snippet cell(r, column)}
				{#if column.id === 'email'}<a href="/users/{encodeURIComponent(r.id)}">{r.email ?? '—'}</a>
				{:else if column.id === 'invites'}{number(r.invites)}
				{:else if column.id === 'joined'}{number(r.joined)}
				{:else if column.id === 'open'}{number(r.open)}
				{:else if column.id === 'joined_later'}{number(r.joined_later)}
				{:else}{minute(r.last_invite)}{/if}
			{/snippet}
		</DataTable>
	{/if}
{/if}

<style>
	.tiles {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(min(100%, 9rem), 1fr));
		border: var(--border-width-regular) solid var(--ink);
		margin-bottom: var(--space-2);
	}
	.tiles div {
		display: grid;
		gap: var(--space-025);
		padding: var(--space-1);
		border-right: var(--border-width-regular) solid var(--ink);
	}
	.tiles div:last-child {
		border-right: 0;
	}
	.tiles .big {
		font-size: var(--type-title-size);
		font-variant-numeric: tabular-nums;
	}
	.tiles span:not(.big) {
		color: var(--text-muted);
		font-size: var(--type-caption-size);
	}
	.bars {
		display: grid;
		gap: var(--space-05);
		margin-bottom: var(--space-2);
		max-width: 40rem;
	}
	.bar {
		display: grid;
		grid-template-columns: 9rem 1fr 3rem;
		gap: var(--space-1);
		align-items: center;
	}
	.track {
		height: 0.9rem;
		border: var(--border-width-hairline) solid var(--ink);
	}
	.fill {
		display: block;
		height: 100%;
		background: var(--text-muted);
	}
	.fill.joined {
		background: var(--ink);
	}
	.nums {
		text-align: right;
		font-variant-numeric: tabular-nums;
	}
</style>
