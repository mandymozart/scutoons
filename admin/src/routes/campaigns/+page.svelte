<script lang="ts">
	import PageHeader from '$lib/components/page-header.svelte';
	import { goto } from '$app/navigation';
	import { ActionBar, Badge, Button, DataTable, Skeleton, askDialog, type Column } from '@scutoons/ui';
	import type { Campaign } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { campaignError, campaignState, stateTone } from '$lib/campaigns.ts';
	import { day, errorText, number } from '$lib/format.ts';
	import { tableLabels } from '$lib/labels.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The campaigns (agents/research/GIFT-INVITES.md): members invite people without an account with a gift. The list
	 * with each one's window and numbers; "New campaign" asks for a name and opens it as a draft.
	 */
	const toaster = getToaster();

	let items = $state<Campaign[] | null>(null);
	let error = $state<string | null>(null);

	admin
		.campaigns()
		.then((r) => (items = r.items))
		.catch((e) => (error = errorText(e)));

	async function create() {
		const name = await askDialog({ title: m['campaigns.new'](), label: m['campaigns.name'](), placeholder: m['campaigns.name_hint'](), confirmLabel: m['campaigns.create']() });
		if (!name?.trim()) return;
		try {
			const r = await admin.createCampaign({ name: name.trim() });
			goto(`/campaigns/${r.item.id}`);
		} catch (e) {
			toaster.addToast({ message: campaignError(e), type: 'error' });
		}
	}

	const columns = $derived<Column[]>([
		{ id: 'name', label: m['campaigns.col_name']() },
		{ id: 'state', label: m['campaigns.col_state']() },
		{ id: 'window', label: m['campaigns.col_window']() },
		{ id: 'sent', label: m['campaigns.col_sent'](), align: 'end' },
		{ id: 'joined', label: m['campaigns.col_joined'](), align: 'end' },
		{ id: 'open', label: m['campaigns.col_open'](), align: 'end', secondary: true }
	]);
</script>

<PageHeader type={m['nav.group_people']()} title={m['campaigns.title']()} meta={m['campaigns.meta']()} />

<ActionBar label={m['shell.page_actions']()} moreLabel={m['shell.more']()}>
	{#snippet end()}
		<Button size="sm" onclick={create}>{m['campaigns.new']()}</Button>
	{/snippet}
</ActionBar>

{#if error}<p class="error-box" role="alert">{error}</p>{/if}

{#if !items}
	{#if !error}<Skeleton variant="table" columns={6} label={m['shell.loading']()} />{/if}
{:else if items.length > 0}
	<DataTable {columns} rows={items} rowKey={(c) => c.id} onrowclick={(c) => goto(`/campaigns/${c.id}`)} labels={tableLabels()}>
		{#snippet cell(c, column)}
			{#if column.id === 'name'}<a href="/campaigns/{c.id}">{c.name}</a>
			{:else if column.id === 'state'}<Badge tone={stateTone(c.state)}>{campaignState(c.state)}</Badge>
			{:else if column.id === 'window'}{day(c.starts_at)} – {day(c.ends_at)}
			{:else if column.id === 'sent'}{number(c.counts.sent)}
			{:else if column.id === 'joined'}{number(c.counts.joined)}
			{:else if column.id === 'open'}{number(c.counts.waiting + c.counts.payment)}{/if}
		{/snippet}
	</DataTable>
{/if}
