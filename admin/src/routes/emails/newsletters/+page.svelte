<script lang="ts">
	import EmailsNav from '$lib/components/emails-nav.svelte';
	import PageHeader from '$lib/components/page-header.svelte';
	import { goto } from '$app/navigation';
	import { onDestroy } from 'svelte';
	import { ActionBar, Badge, Button, DataTable, Skeleton, askDialog, type Column } from '@scutoons/ui';
	import type { Newsletter, NewsletterBatchResult, NewsletterState } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { day, errorText, number } from '$lib/format.ts';
	import { tableLabels } from '$lib/labels.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { newsletterStatus, statusTone } from '$lib/newsletter.ts';

	/**
	 * The newsletters (agents/NEWSLETTER.md): the list with each one's progress, the sending state (waiting mails, the
	 * hourly limit, cron or not) and "Send now" – it asks the API for one batch after the other while the page is open
	 * (the host's cron does the same every few minutes once NEWSLETTER_CRON_KEY is set).
	 */
	const toaster = getToaster();

	let items = $state<Newsletter[] | null>(null);
	let sendState = $state<NewsletterState | null>(null);
	let error = $state<string | null>(null);
	let sending = $state(false);
	let destroyed = false;
	onDestroy(() => (destroyed = true));

	function load() {
		admin
			.newsletters()
			.then((r) => (items = r.items))
			.catch((e) => (error = errorText(e)));
		admin
			.newsletterState()
			.then((r) => (sendState = r))
			.catch(() => {});
	}
	load();

	async function create() {
		const title = await askDialog({ title: m['newsletter.new'](), label: m['newsletter.new_title'](), placeholder: m['newsletter.new_title_hint'](), confirmLabel: m['newsletter.create']() });
		if (!title?.trim()) return;
		try {
			const r = await admin.createNewsletter({ title: title.trim(), audiences: ['members_opted_in'] });
			goto(`/emails/newsletters/${r.item.id}`);
		} catch (e) {
			toaster.addToast({ message: errorText(e), type: 'error' });
		}
	}

	const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

	/** one batch after the other until nothing waits, the hourly limit is reached, an error or "Stop" */
	async function sendNow() {
		sending = true;
		let total = 0;
		try {
			while (sending && !destroyed) {
				const r = await admin.sendNewsletterBatch();
				items = r.items;
				const result: NewsletterBatchResult = r.result;
				total += result.sent;
				if (sendState) sendState = { ...sendState, queued: result.queued, sent_last_hour: result.sent_last_hour, per_hour: result.per_hour };
				if (result.status === 'error') {
					toaster.addToast({ message: m['newsletter.batch_error']({ error: result.error ?? '' }), type: 'error' });
					break;
				}
				if (result.status === 'limit') {
					toaster.addToast({ message: m['newsletter.batch_limit'](), type: 'info' });
					break;
				}
				if (result.status === 'idle' || (result.status === 'sent' && result.queued === 0)) break;
				await wait(result.status === 'busy' ? 5000 : 1500);
			}
		} catch (e) {
			toaster.addToast({ message: errorText(e), type: 'error' });
		} finally {
			sending = false;
			if (total > 0) toaster.addToast({ message: m['newsletter.batch_sent']({ count: number(total) }), type: 'success' });
		}
	}

	const columns = $derived<Column[]>([
		{ id: 'title', label: m['newsletter.col_title']() },
		{ id: 'status', label: m['newsletter.col_status']() },
		{ id: 'progress', label: m['newsletter.col_progress']() },
		{ id: 'started_at', label: m['newsletter.col_started'](), secondary: true }
	]);
</script>

<PageHeader type={m['nav.group_system']()} title={m['newsletter.title']()} meta={m['newsletter.meta']()} />
<EmailsNav current="newsletters" />

{#snippet barStart()}
	{#if sendState}
		<span class="state">
			{sendState.per_hour > 0
				? m['newsletter.state_line']({ queued: number(sendState.queued), sent: number(sendState.sent_last_hour), limit: number(sendState.per_hour) })
				: `${m['newsletter.state_line']({ queued: number(sendState.queued), sent: number(sendState.sent_last_hour), limit: '∞' })} (${m['newsletter.state_unlimited']()})`}
		</span>
	{/if}
{/snippet}

<ActionBar start={barStart} label={m['shell.page_actions']()} moreLabel={m['shell.more']()}>
	{#snippet end()}
		{#if sending}
			<span class="state">{m['newsletter.sending_now']()}</span>
			<Button size="sm" variant="secondary" onclick={() => (sending = false)}>{m['newsletter.stop_sending']()}</Button>
		{:else if sendState && sendState.queued > 0}
			<Button size="sm" variant="secondary" onclick={sendNow}>{m['newsletter.send_now']()}</Button>
		{/if}
		<Button size="sm" onclick={create}>{m['newsletter.new']()}</Button>
	{/snippet}
</ActionBar>

{#if sendState?.transport === 'log'}
	<p class="note">{m['newsletter.state_log']()}</p>
{:else if sendState}
	<p class="note">{sendState.cron ? m['newsletter.state_cron']() : m['newsletter.state_no_cron']()}</p>
{/if}

{#if error}<p class="error-box" role="alert">{error}</p>{/if}

{#if !items}
	<Skeleton variant="table" columns={4} label={m['shell.loading']()} />
{:else if items.length > 0}
	<DataTable {columns} rows={items} rowKey={(n) => String(n.id)} onrowclick={(n) => goto(`/emails/newsletters/${n.id}`)} labels={tableLabels()}>
		{#snippet cell(n, column)}
			{#if column.id === 'title'}<a href="/emails/newsletters/{n.id}">{n.title}</a>
			{:else if column.id === 'status'}<Badge tone={statusTone(n.status)}>{newsletterStatus(n.status)}</Badge>
				{#if n.last_error}<Badge tone="error"><span title={n.last_error}>{m['newsletter.col_error']()}</span></Badge>{/if}
			{:else if column.id === 'progress'}
				{#if n.progress.total > 0}
					<span class="nums">{m['newsletter.progress']({ sent: number(n.progress.sent), total: number(n.progress.total) })}</span>
					{#if n.progress.failed > 0}<span class="muted"> · {m['newsletter.progress_failed']({ count: number(n.progress.failed) })}</span>{/if}
				{:else}—{/if}
			{:else if column.id === 'started_at'}{day(n.started_at)}{/if}
		{/snippet}
	</DataTable>
{/if}

<style>
	.state {
		align-self: center;
		color: var(--text-muted);
	}
	.note {
		margin: 0 0 var(--space-1);
		color: var(--text-muted);
	}
</style>
