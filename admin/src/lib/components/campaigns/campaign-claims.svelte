<script lang="ts">
	import { Badge, Button, DataTable, Skeleton, type Column } from '@scutoons/ui';
	import type { Campaign, CampaignClaims } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { campaignError, claimStatus, claimTone } from '$lib/campaigns.ts';
	import { day, number } from '$lib/format.ts';
	import { tableLabels } from '$lib/labels.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A "Verschenken" campaign's claims (#66, agents/research/CAMPAIGN-GIFTS.md §4): how many were claimed, and the
	 * physical gifts – "to send" (with the account's shipping address) first, then waiting for an address, sent, lapsed.
	 */
	let { campaign }: { campaign: Campaign } = $props();
	const toaster = getToaster();

	let data = $state<CampaignClaims | null>(null);
	let busy = $state<number | null>(null);

	$effect(() => {
		admin
			.campaignClaims(campaign.id)
			.then((r) => (data = r))
			.catch((e) => toaster.addToast({ message: campaignError(e), type: 'error' }));
	});

	async function shipped(claimId: number) {
		busy = claimId;
		try {
			data = await admin.claimShipped(claimId);
		} catch (e) {
			toaster.addToast({ message: campaignError(e), type: 'error' });
		} finally {
			busy = null;
		}
	}

	const columns = $derived<Column[]>([
		{ id: 'status', label: m['campaigns.claim_col_status']() },
		{ id: 'to', label: m['campaigns.claim_col_to']() },
		{ id: 'address', label: m['campaigns.claim_col_address']() },
		{ id: 'claimed', label: m['campaigns.claim_col_claimed'](), secondary: true },
		{ id: 'action', label: '', align: 'end' }
	]);
</script>

{#if !data}
	<Skeleton lines={4} label={m['shell.loading']()} />
{:else}
	<p class="counts">
		{m['campaigns.claims_counts']({ claimed: number(campaign.counts.claimed), left: campaign.stock_left === null ? '∞' : number(campaign.stock_left) })}
	</p>
	{#if data.physical.length > 0}
		<DataTable {columns} rows={data.physical} rowKey={(c) => String(c.id)} labels={tableLabels()}>
			{#snippet cell(c, column)}
				{#if column.id === 'status'}<Badge tone={claimTone(c.status)}>{claimStatus(c.status)}</Badge>
				{:else if column.id === 'to'}{c.ship_name ?? c.email}
				{:else if column.id === 'address'}{#if c.ship_address}<span class="address">{c.ship_address}</span> · {c.ship_country}{:else}<span class="muted">—</span>{/if}
				{:else if column.id === 'claimed'}{day(c.created_at)}
				{:else if column.id === 'action' && c.status === 'ship'}
					<Button size="sm" disabled={busy === c.id} onclick={() => shipped(c.id)}>{m['campaigns.claim_shipped']()}</Button>
				{/if}
			{/snippet}
		</DataTable>
	{/if}
{/if}

<style>
	.counts {
		margin: 0 0 var(--space-1);
	}
	.address {
		white-space: pre-line;
	}
</style>
