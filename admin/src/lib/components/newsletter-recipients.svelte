<script lang="ts">
	import { Badge, DataTable, Section, type Column } from '@scutoons/ui';
	import type { ListResult, Newsletter, NewsletterRecipient } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { minute, number } from '$lib/format.ts';
	import { tableLabels } from '$lib/labels.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The recipients of a started newsletter (agents/NEWSLETTER.md "Sending"): each address with its status, filtered by
	 * status; loads again when the newsletter changes (a batch, start / pause / resume – the page hands the new item).
	 */
	let { item }: { item: Newsletter } = $props();

	let status = $state<string>('');
	let recipients = $state<ListResult<NewsletterRecipient> | null>(null);

	$effect(() => {
		const id = item.id;
		void item.progress.sent;
		const filter = status;
		admin
			.newsletterRecipients(id, { page_size: 200, sort: 'email', ...(filter ? { status: filter } : {}) })
			.then((r) => (recipients = r))
			.catch(() => {});
	});

	const columns = $derived<Column[]>([
		{ id: 'email', label: m['newsletter.col_email']() },
		{ id: 'status', label: m['newsletter.col_status']() },
		{ id: 'locale', label: m['newsletter.col_language'](), secondary: true },
		{ id: 'source', label: m['newsletter.col_source'](), secondary: true },
		{ id: 'sent_at', label: m['newsletter.col_sent'](), secondary: true },
		{ id: 'error', label: m['newsletter.col_error']() }
	]);
	const label = (s: string) =>
		({ queued: m['newsletter.r_queued'](), sent: m['newsletter.r_sent'](), failed: m['newsletter.r_failed'](), skipped: m['newsletter.r_skipped']() })[s] ?? s;
</script>

<Section title={m['newsletter.recipients']()}>
	<label class="bar-filter">
		<span>{m['newsletter.col_status']()}</span>
		<select bind:value={status}>
			<option value="">{m['common.all']()}</option>
			{#each ['queued', 'sent', 'failed', 'skipped'] as s (s)}
				<option value={s}>{label(s)} ({number(item.progress[s as 'queued'])})</option>
			{/each}
		</select>
	</label>
	{#if recipients}
		<DataTable {columns} rows={recipients.items} rowKey={(r) => String(r.id)} labels={tableLabels()}>
			{#snippet cell(r, column)}
				{#if column.id === 'email'}{r.email}
				{:else if column.id === 'status'}<Badge tone={r.status === 'failed' ? 'error' : r.status === 'sent' ? 'neutral' : 'draft'}>{label(r.status)}</Badge>
				{:else if column.id === 'locale'}{r.locale}
				{:else if column.id === 'source'}{r.source === 'member' ? m['newsletter.source_member']() : m['newsletter.source_contact']()}
				{:else if column.id === 'sent_at'}{minute(r.sent_at)}
				{:else if column.id === 'error'}<span class="error-text">{r.error ?? ''}</span>{/if}
			{/snippet}
		</DataTable>
	{/if}
</Section>

<style>
	.error-text {
		overflow-wrap: anywhere;
	}
</style>
