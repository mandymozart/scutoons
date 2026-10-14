<script lang="ts">
	import { untrack } from 'svelte';
	import { Badge, Button, DataTable, Skeleton, askDialog, confirmDialog, type Column } from '@scutoons/ui';
	import type { AdminCampaignInvite, Campaign, InviteStatus } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { campaignError, inviteStatus, inviteTone, money } from '$lib/campaigns.ts';
	import { day, minute, number } from '$lib/format.ts';
	import { tableLabels } from '$lib/labels.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A campaign's invites (agents/research/GIFT-INVITES.md): every invite with who, to whom, what, its state. Two jobs
	 * for the admin: "Payment open" – the member pays a physical gift; send them a payment link or the bank details by
	 * hand, then "Payment received" sends the invite – and "To ship": a joined friend's address, "Shipped" when it left.
	 */
	let { campaign }: { campaign: Campaign } = $props();

	const toaster = getToaster();
	const FILTERS: (InviteStatus | 'to_ship' | '')[] = ['', 'payment', 'to_ship', 'waiting', 'joined', 'returned', 'recalled', 'unpaid'];
	// starts on the open payments when there are any (then the admin's choice)
	let status = $state<InviteStatus | 'to_ship' | ''>(untrack(() => (campaign.counts.payment > 0 ? 'payment' : '')));
	let items = $state<AdminCampaignInvite[] | null>(null);
	let busy = $state(false);

	function load() {
		admin
			.campaignInvites(campaign.id, status || undefined)
			.then((r) => (items = r.items))
			.catch((e) => toaster.addToast({ message: campaignError(e), type: 'error' }));
	}
	$effect(() => {
		void status;
		load();
	});

	async function paid(invite: AdminCampaignInvite) {
		const ok = await confirmDialog({
			title: m['campaigns.paid_title']({ price: money(invite.price_cents, invite.currency ?? 'EUR') }),
			message: m['campaigns.paid_message']({ member: invite.inviter.email ?? '', friend: invite.email }),
			confirmLabel: m['campaigns.paid']()
		});
		if (!ok) return;
		await run(() => admin.invitePaid(invite.id), m['campaigns.paid_done']({ friend: invite.email }));
	}

	async function shipped(invite: AdminCampaignInvite) {
		const note = await askDialog({ title: m['campaigns.shipped_title']({ friend: invite.email }), label: m['campaigns.shipped_note'](), placeholder: m['campaigns.shipped_note_hint'](), confirmLabel: m['campaigns.shipped']() });
		if (note === null) return;
		await run(() => admin.inviteShipped(invite.id, note.trim() || undefined), m['campaigns.shipped_done']());
	}

	async function run(action: () => Promise<unknown>, done: string) {
		busy = true;
		try {
			await action();
			toaster.addToast({ message: done, type: 'success' });
			load();
		} catch (e) {
			toaster.addToast({ message: campaignError(e), type: 'error' });
		} finally {
			busy = false;
		}
	}

	const columns = $derived<Column[]>([
		{ id: 'email', label: m['campaigns.col_friend']() },
		{ id: 'inviter', label: m['campaigns.col_inviter']() },
		{ id: 'item', label: m['campaigns.col_item']() },
		{ id: 'status', label: m['campaigns.col_status']() },
		{ id: 'when', label: m['campaigns.col_when'](), secondary: true },
		{ id: 'action', label: '' }
	]);
</script>

<div class="invites">
	<label class="bar-filter">
		<span>{m['campaigns.col_status']()}</span>
		<select bind:value={status}>
			{#each FILTERS as s (s)}<option value={s}>{s === '' ? m['common.all']() : inviteStatus(s)}</option>{/each}
		</select>
	</label>
	{#if status === 'payment'}<p class="muted">{m['campaigns.payment_hint']()}</p>{/if}
	{#if status === 'to_ship'}<p class="muted">{m['campaigns.to_ship_hint']()}</p>{/if}

	{#if !items}
		<Skeleton variant="table" columns={5} label={m['shell.loading']()} />
	{:else if items.length > 0}
		<DataTable {columns} rows={items} rowKey={(i) => i.id} labels={tableLabels()}>
			{#snippet cell(i, column)}
				{#if column.id === 'email'}
					{#if i.joined}<a href="/users/{encodeURIComponent(i.joined.id)}">{i.email}</a>{:else}{i.email}{/if}
					{#if i.ship}
						<div class="ship">{i.ship.name}<br />{i.ship.address}<br />{i.ship.country}</div>
					{/if}
				{:else if column.id === 'inviter'}
					<a href="/users/{encodeURIComponent(i.inviter.id)}">{i.inviter.email ?? '—'}</a>
				{:else if column.id === 'item'}
					{i.qty > 1 ? `${number(i.qty)} × ` : ''}{i.item}
					{#if i.physical}<Badge tone="ink">{m['campaigns.physical']()}</Badge>{/if}
					{#if i.price_cents !== null}<span class="muted"> · {money(i.price_cents, i.currency ?? 'EUR')}</span>{/if}
				{:else if column.id === 'status'}
					<Badge tone={inviteTone(i.status)}>{inviteStatus(i.status)}</Badge>
					{#if i.shipped_at}<Badge tone="neutral">{m['campaigns.shipped_badge']({ date: day(i.shipped_at) })}</Badge>{/if}
				{:else if column.id === 'when'}
					{i.status === 'payment' ? m['campaigns.due']({ date: day(i.payment_due_at) }) : i.status === 'waiting' ? m['campaigns.until']({ date: day(i.expires_at) }) : minute(i.joined_at ?? i.ended_at ?? i.created_at)}
				{:else if column.id === 'action'}
					{#if i.status === 'payment'}
						<Button size="sm" disabled={busy} onclick={() => paid(i)}>{m['campaigns.paid']()}</Button>
					{:else if i.status === 'joined' && i.physical && i.ship && !i.shipped_at}
						<Button size="sm" disabled={busy} onclick={() => shipped(i)}>{m['campaigns.shipped']()}</Button>
					{:else if i.status === 'joined' && i.physical && !i.ship}
						<span class="muted">{m['campaigns.no_address']()}</span>
					{/if}
				{/if}
			{/snippet}
		</DataTable>
	{/if}
</div>

<style>
	/* one column that may shrink: a wide table scrolls inside it instead of widening the page */
	.invites {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		gap: var(--space-1);
	}
	.invites > p {
		margin: 0;
	}
	.ship {
		margin-top: var(--space-025);
		white-space: pre-line;
		color: var(--text-muted);
	}
</style>
