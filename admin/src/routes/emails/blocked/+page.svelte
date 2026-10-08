<script lang="ts">
	import EmailsNav from '$lib/components/emails-nav.svelte';
	import PageHeader from '$lib/components/page-header.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { ActionBar, Button, DataTable, Pager, SearchField, Skeleton, askDialog, confirmDialog, listHref, listParams, readList, type Column } from '@scutoons/ui';
	import type { ListResult, NewsletterSuppression } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { errorText, minute, number } from '$lib/format.ts';
	import { pagerLabels, tableLabels } from '$lib/labels.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * Addresses no newsletter goes to (agents/NEWSLETTER.md "Unsubscribe and bounces"): unsubscribed by the link,
	 * bounced, or blocked here by hand. "Mail again" only when the person asked for it.
	 */
	const toaster = getToaster();
	const query = $derived(readList(page.url, { pageSize: 100, sort: '-created_at', filters: ['reason'] }));

	let result = $state<ListResult<NewsletterSuppression> | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(false);
	let reload = $state(0);

	$effect(() => {
		void reload;
		const params = listParams(query);
		loading = true;
		admin
			.newsletterSuppressions(params)
			.then((r) => (result = r))
			.catch((e) => (error = errorText(e)))
			.finally(() => (loading = false));
	});

	const href = (patch: Record<string, string | number | null>) => listHref(page.url, patch);
	const nav = (patch: Record<string, string | number | null>) => goto(href({ page: null, ...patch }), { keepFocus: true, noScroll: true });
	const reasonLabel = (reason: string) =>
		({ unsubscribe: m['newsletter.reason_unsubscribe'](), bounce: m['newsletter.reason_bounce'](), manual: m['newsletter.reason_manual']() })[reason] ?? reason;

	async function block() {
		const email = await askDialog({ title: m['newsletter.block'](), label: m['newsletter.block_label'](), placeholder: 'name@example.com', confirmLabel: m['newsletter.block']() });
		if (!email?.trim()) return;
		try {
			await admin.newsletterSuppress(email.trim());
			toaster.addToast({ message: m['newsletter.blocked_added'](), type: 'success' });
			reload++;
		} catch (e) {
			toaster.addToast({ message: errorText(e), type: 'error' });
		}
	}

	async function unblock(email: string) {
		if (!(await confirmDialog({ title: m['newsletter.unblock_title']({ email }), message: m['newsletter.unblock_message'](), confirmLabel: m['newsletter.unblock']() }))) return;
		try {
			await admin.newsletterUnsuppress(email);
			toaster.addToast({ message: m['newsletter.unblocked']({ email }), type: 'success' });
			reload++;
		} catch (e) {
			toaster.addToast({ message: errorText(e), type: 'error' });
		}
	}

	const columns = $derived<Column[]>([
		{ id: 'email', label: m['newsletter.col_email'](), sortable: true },
		{ id: 'reason', label: m['newsletter.reason'](), sortable: true },
		{ id: 'created_at', label: m['newsletter.col_when'](), sortable: true, secondary: true },
		{ id: 'detail', label: m['newsletter.col_detail'](), secondary: true },
		{ id: 'actions', label: '' }
	]);
</script>

<PageHeader type={m['nav.group_system']()} title={m['newsletter.blocked_title']()} meta={result ? m['newsletter.blocked_count']({ count: number(result.pagination.total) }) : m['newsletter.blocked_meta']()} />
<EmailsNav current="blocked" />

{#snippet barStart()}
	<SearchField compact label={m['newsletter.search']()} placeholder="name@example.com" value={query.q} onsearch={(q) => nav({ q })} />
	<label class="bar-filter">
		<span>{m['newsletter.reason']()}</span>
		<select value={query.filters.reason ?? ''} onchange={(e) => nav({ reason: e.currentTarget.value })}>
			<option value="">{m['common.all']()}</option>
			{#each ['unsubscribe', 'bounce', 'manual'] as r (r)}<option value={r}>{reasonLabel(r)}</option>{/each}
		</select>
	</label>
	{#if result}
		<Pager compact page={result.pagination.page} pageSize={result.pagination.pageSize} total={result.pagination.total} href={(p) => href({ page: p })} labels={pagerLabels()} />
	{/if}
{/snippet}

<ActionBar start={barStart} label={m['shell.page_actions']()} moreLabel={m['shell.more']()}>
	{#snippet end()}
		<Button size="sm" variant="secondary" onclick={block}>{m['newsletter.block']()}</Button>
	{/snippet}
</ActionBar>

<p class="muted">{m['newsletter.blocked_meta']()}</p>
{#if error}<p class="error-box" role="alert">{error}</p>{/if}

{#if !result}
	<Skeleton variant="table" columns={4} label={m['shell.loading']()} />
{:else}
	<DataTable {columns} rows={result.items} rowKey={(s) => s.email} sort={query.sort} sortHref={(sort) => href({ sort })} {loading} labels={tableLabels()}>
		{#snippet cell(s, column)}
			{#if column.id === 'email'}{s.email}
			{:else if column.id === 'reason'}{reasonLabel(s.reason)}
			{:else if column.id === 'created_at'}{minute(s.created_at)}
			{:else if column.id === 'detail'}<span class="detail">{s.detail ?? ''}</span>
			{:else if column.id === 'actions'}<Button size="sm" variant="clear" onclick={() => unblock(s.email)}>{m['newsletter.unblock']()}</Button>{/if}
		{/snippet}
	</DataTable>
{/if}

<style>
	.detail {
		overflow-wrap: anywhere;
	}
</style>
