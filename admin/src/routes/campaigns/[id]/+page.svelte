<script lang="ts">
	import PageHeader from '$lib/components/page-header.svelte';
	import CampaignSettings from '$lib/components/campaigns/campaign-settings.svelte';
	import CampaignAccounts from '$lib/components/campaigns/campaign-accounts.svelte';
	import CampaignInvites from '$lib/components/campaigns/campaign-invites.svelte';
	import CampaignStats from '$lib/components/campaigns/campaign-stats.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { ActionBar, Badge, Button, Skeleton, Tabs, confirmDialog } from '@scutoons/ui';
	import type { Campaign } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { campaignError, campaignState, stateTone } from '$lib/campaigns.ts';
	import { day, errorText, number } from '$lib/format.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * One campaign (agents/research/GIFT-INVITES.md): Settings (window, gift, the invitees' reading, limits), Who may
	 * invite (all / ticked accounts / a segment, with the account filters), Invites (payments to clear, gifts to ship)
	 * and Statistics. Start / stop in the bar; a draft without invites can be deleted.
	 */
	const toaster = getToaster();
	const id = $derived(page.params.id ?? '');
	const TABS = ['settings', 'inviters', 'invites', 'stats'] as const;
	type TabId = (typeof TABS)[number];

	let item = $state<Campaign | null>(null);
	let error = $state<string | null>(null);
	let busy = $state(false);
	let tab = $state<TabId>('settings');

	// the tab from the URL (?tab=invites – the payment mail links there)
	$effect(() => {
		const wanted = page.url.searchParams.get('tab');
		if (wanted && (TABS as readonly string[]).includes(wanted)) tab = wanted as TabId;
	});

	$effect(() => {
		const current = id;
		error = null;
		admin
			.getCampaign(current)
			.then((r) => (item = r.item))
			.catch((e) => (error = errorText(e)));
	});

	async function setStatus(action: 'start' | 'stop') {
		if (!item) return;
		if (action === 'start') {
			const ok = await confirmDialog({ title: m['campaigns.start_title'](), message: m['campaigns.start_message']({ from: day(item.starts_at), until: day(item.ends_at) }), confirmLabel: m['campaigns.start']() });
			if (!ok) return;
		} else {
			const ok = await confirmDialog({ title: m['campaigns.stop_title'](), message: m['campaigns.stop_message'](), confirmLabel: m['campaigns.stop']() });
			if (!ok) return;
		}
		busy = true;
		try {
			item = (action === 'start' ? await admin.startCampaign(item.id) : await admin.stopCampaign(item.id)).item;
		} catch (e) {
			toaster.addToast({ message: campaignError(e), type: 'error' });
		} finally {
			busy = false;
		}
	}

	async function remove() {
		if (!item) return;
		if (!(await confirmDialog({ title: m['campaigns.delete_title'](), message: m['campaigns.delete_message'](), confirmLabel: m['campaigns.delete']() }))) return;
		try {
			await admin.deleteCampaign(item.id);
			goto('/campaigns');
		} catch (e) {
			toaster.addToast({ message: campaignError(e), type: 'error' });
		}
	}

	const tabs = $derived([
		{ id: 'settings', label: m['campaigns.tab_settings']() },
		{ id: 'inviters', label: m['campaigns.tab_inviters']() },
		{ id: 'invites', label: m['campaigns.tab_invites'](), count: item?.counts.sent },
		{ id: 'stats', label: m['campaigns.tab_stats']() }
	]);
</script>

<PageHeader type={m['campaigns.title']()} title={item?.name ?? '…'} meta={item ? m['campaigns.meta_counts']({ sent: number(item.counts.sent), joined: number(item.counts.joined) }) : undefined} />

{#snippet barStart()}
	<a href="/campaigns">{m['campaigns.back']()}</a>
	{#if item}<Badge tone={stateTone(item.state)}>{campaignState(item.state)}</Badge>{/if}
{/snippet}

<ActionBar start={barStart} label={m['shell.page_actions']()} moreLabel={m['shell.more']()}>
	{#snippet secondary()}
		{#if item && item.counts.sent === 0}
			<Button size="sm" variant="secondary" onclick={remove}>{m['campaigns.delete']()}</Button>
		{/if}
	{/snippet}
	{#snippet end()}
		{#if item}
			{#if item.status === 'started'}
				<Button size="sm" variant="secondary" disabled={busy} onclick={() => setStatus('stop')}>{m['campaigns.stop']()}</Button>
			{:else}
				<Button size="sm" disabled={busy} onclick={() => setStatus('start')}>{m['campaigns.start']()}</Button>
			{/if}
		{/if}
	{/snippet}
</ActionBar>

{#if error}<p class="error-box" role="alert">{error}</p>{/if}

{#if !item}
	{#if !error}<Skeleton lines={8} label={m['shell.loading']()} />{/if}
{:else}
	<Tabs {tabs} value={tab} onchange={(v) => (tab = v as TabId)} label={m['campaigns.title']()} variant="ruled" />
	<div class="panel">
		{#if tab === 'settings'}
			<CampaignSettings campaign={item} onsaved={(next) => (item = next)} />
		{:else if tab === 'inviters'}
			<CampaignAccounts campaign={item} />
		{:else if tab === 'invites'}
			<CampaignInvites campaign={item} />
		{:else}
			<CampaignStats campaign={item} />
		{/if}
	</div>
{/if}

<style>
	.panel {
		margin-top: var(--space-2);
	}
</style>
