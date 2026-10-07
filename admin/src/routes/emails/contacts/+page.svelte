<script lang="ts">
	import EmailsNav from '$lib/components/emails-nav.svelte';
	import PageHeader from '$lib/components/page-header.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { ActionBar, Badge, Button, DataTable, Field, Pager, SearchField, Sheet, Skeleton, confirmDialog, listHref, listParams, readList, type Column } from '@scutoons/ui';
	import type { ListResult, NewsletterContact, NewsletterImportResult, NewsletterImportStatus, NewsletterLocale, NewsletterSource } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { day, errorText, number } from '$lib/format.ts';
	import { pagerLabels, tableLabels } from '$lib/labels.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * Newsletter contacts (agents/NEWSLETTER.md "Contacts"): the addresses that are no account – the old QR form's
	 * (source qr-claim) and the event lists. URL-driven list (source, language incl. "not set"), the language for several
	 * at once, delete, and the import (paste the list → check → import the new ones).
	 */
	const toaster = getToaster();
	const query = $derived(readList(page.url, { pageSize: 100, sort: '-created_at', filters: ['source', 'locale'] }));

	let result = $state<(ListResult<NewsletterContact> & { sources: NewsletterSource[] }) | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(false);
	let selected = $state<string[]>([]);
	let reload = $state(0);

	$effect(() => {
		void reload;
		const params = listParams(query);
		loading = true;
		error = null;
		admin
			.newsletterContacts(params)
			.then((r) => (result = r))
			.catch((e) => (error = errorText(e)))
			.finally(() => (loading = false));
	});

	const href = (patch: Record<string, string | number | null>) => listHref(page.url, patch);
	const nav = (patch: Record<string, string | number | null>) => goto(href({ page: null, ...patch }), { keepFocus: true, noScroll: true });

	async function setLanguage(keys: string[], locale: NewsletterLocale) {
		try {
			const r = await admin.newsletterUpdateContacts(keys.map(Number), { locale });
			toaster.addToast({ message: m['newsletter.language_set']({ count: number(r.changed) }), type: 'success' });
			selected = [];
			reload++;
		} catch (e) {
			toaster.addToast({ message: errorText(e), type: 'error' });
		}
	}

	async function setOne(contact: NewsletterContact, locale: string) {
		try {
			await admin.newsletterUpdateContact(contact.id, { locale: (locale || null) as NewsletterLocale | null });
			reload++;
		} catch (e) {
			toaster.addToast({ message: errorText(e), type: 'error' });
		}
	}

	async function remove(keys: string[]) {
		if (!(await confirmDialog({ title: m['newsletter.delete_contacts_title']({ count: number(keys.length) }), confirmLabel: m['newsletter.delete_contacts']() }))) return;
		try {
			const r = await admin.newsletterDeleteContacts(keys.map(Number));
			toaster.addToast({ message: m['newsletter.contacts_deleted']({ count: number(r.deleted) }), type: 'success' });
			selected = [];
			reload++;
		} catch (e) {
			toaster.addToast({ message: errorText(e), type: 'error' });
		}
	}

	// the import: paste → check (dry run) → import the new ones
	let importOpen = $state(false);
	let importText = $state('');
	let importSource = $state('event:');
	let importDate = $state('');
	let importLocale = $state<'' | NewsletterLocale>('de');
	let check = $state<NewsletterImportResult | null>(null);
	let importing = $state(false);
	const importInput = $derived({ text: importText, source: importSource.trim(), consented_at: importDate || null, locale: importLocale || null });
	// a change after the check needs a new check
	$effect(() => {
		void importInput.text;
		void importInput.source;
		void importInput.locale;
		check = null;
	});

	function openImport() {
		importText = '';
		importSource = 'event:';
		importDate = '';
		check = null;
		importOpen = true;
	}

	async function runImport(dryRun: boolean) {
		importing = true;
		try {
			const r = await admin.newsletterImport({ ...importInput, dry_run: dryRun });
			if (dryRun) {
				check = r;
				return;
			}
			toaster.addToast({ message: m['newsletter.import_done']({ count: number(r.added) }), type: 'success' });
			importOpen = false;
			reload++;
		} catch (e) {
			toaster.addToast({ message: errorText(e), type: 'error' });
		} finally {
			importing = false;
		}
	}

	const importLabel = (status: NewsletterImportStatus) =>
		({
			new: m['newsletter.i_new'](),
			exists: m['newsletter.i_exists'](),
			member: m['newsletter.i_member'](),
			suppressed: m['newsletter.i_suppressed'](),
			duplicate: m['newsletter.i_duplicate'](),
			invalid: m['newsletter.i_invalid']()
		})[status];

	const columns = $derived<Column[]>([
		{ id: 'email', label: m['newsletter.col_email'](), sortable: true },
		{ id: 'name', label: m['newsletter.col_name'](), sortable: true, secondary: true },
		{ id: 'locale', label: m['newsletter.col_language'](), sortable: true },
		{ id: 'source', label: m['newsletter.col_source'](), sortable: true },
		{ id: 'consented_at', label: m['newsletter.col_consented'](), secondary: true }
	]);
</script>

<PageHeader type={m['nav.group_system']()} title={m['newsletter.contacts_title']()} meta={result ? m['newsletter.contacts_count']({ count: number(result.pagination.total) }) : m['newsletter.contacts_meta']()} />
<EmailsNav current="contacts" />

{#snippet barStart()}
	<SearchField compact label={m['newsletter.search']()} placeholder={m['newsletter.search_placeholder']()} value={query.q} onsearch={(q) => nav({ q })} />
	<label class="bar-filter">
		<span>{m['newsletter.filter_source']()}</span>
		<select value={query.filters.source ?? ''} onchange={(e) => nav({ source: e.currentTarget.value })}>
			<option value="">{m['common.all']()}</option>
			{#each result?.sources ?? [] as s (s.source)}
				<option value={s.source}>{s.source} ({number(s.count)})</option>
			{/each}
		</select>
	</label>
	<label class="bar-filter">
		<span>{m['newsletter.filter_language']()}</span>
		<select value={query.filters.locale ?? ''} onchange={(e) => nav({ locale: e.currentTarget.value })}>
			<option value="">{m['common.all']()}</option>
			<option value="de">de</option>
			<option value="en">en</option>
			<option value="none">{m['newsletter.language_none']()}</option>
		</select>
	</label>
	{#if query.q || Object.keys(query.filters).length}
		<a href="/emails/contacts">{m['common.reset_filters']()}</a>
	{/if}
	{#if result}
		<Pager compact page={result.pagination.page} pageSize={result.pagination.pageSize} total={result.pagination.total} href={(p) => href({ page: p })} labels={pagerLabels()} />
	{/if}
{/snippet}

<ActionBar start={barStart} label={m['shell.page_actions']()} moreLabel={m['shell.more']()}>
	{#snippet end()}
		<Button size="sm" onclick={openImport}>{m['newsletter.import']()}</Button>
	{/snippet}
</ActionBar>

{#if error}<p class="error-box" role="alert">{error}</p>{/if}

{#if !result}
	<Skeleton variant="table" columns={5} label={m['shell.loading']()} />
{:else}
	<DataTable
		{columns}
		rows={result.items}
		rowKey={(c) => String(c.id)}
		sort={query.sort}
		sortHref={(sort) => href({ sort })}
		selectable
		bind:selected
		{loading}
		labels={tableLabels()}
	>
		{#snippet cell(c, column)}
			{#if column.id === 'email'}{c.email}
				{#if c.member}<Badge tone="info">{m['newsletter.member']()}</Badge>{/if}
				{#if c.suppressed}<Badge tone="warning">{m['newsletter.i_suppressed']()}</Badge>{/if}
			{:else if column.id === 'name'}{c.name ?? ''}
			{:else if column.id === 'locale'}
				<select class="locale" class:missing={!c.locale} value={c.locale ?? ''} aria-label={m['newsletter.col_language']()} onchange={(e) => setOne(c, e.currentTarget.value)}>
					<option value="">{m['newsletter.language_none']()}</option>
					<option value="de">de</option>
					<option value="en">en</option>
				</select>
			{:else if column.id === 'source'}{c.source}
			{:else if column.id === 'consented_at'}{day(c.consented_at)}{/if}
		{/snippet}
		{#snippet bulk(keys)}
			<Button size="sm" variant="secondary" onclick={() => setLanguage(keys, 'de')}>{m['newsletter.set_language']({ locale: 'de' })}</Button>
			<Button size="sm" variant="secondary" onclick={() => setLanguage(keys, 'en')}>{m['newsletter.set_language']({ locale: 'en' })}</Button>
			<Button size="sm" variant="secondary" onclick={() => remove(keys)}>{m['newsletter.delete_contacts']()}</Button>
		{/snippet}
	</DataTable>
{/if}

<Sheet bind:open={importOpen} side="right" widthKey="newsletter-import" title={m['newsletter.import_title']()} closeLabel={m['common.close']()}>
	<div class="import">
		<Field label={m['newsletter.import_text']()} hint={m['newsletter.import_text_hint']()}>
			{#snippet children({ id, describedby })}<textarea {id} aria-describedby={describedby} rows="10" bind:value={importText}></textarea>{/snippet}
		</Field>
		<Field label={m['newsletter.import_source']()} hint={m['newsletter.import_source_hint']()}>
			{#snippet children({ id, describedby })}<input {id} aria-describedby={describedby} maxlength="100" bind:value={importSource} />{/snippet}
		</Field>
		<Field label={m['newsletter.import_date']()}>
			{#snippet children({ id })}<input {id} type="date" bind:value={importDate} />{/snippet}
		</Field>
		<Field label={m['newsletter.import_language']()}>
			{#snippet children({ id })}
				<select {id} bind:value={importLocale}>
					<option value="de">de</option>
					<option value="en">en</option>
					<option value="">{m['newsletter.language_none']()}</option>
				</select>
			{/snippet}
		</Field>
		{#if check}
			<p class="counts">
				{#each Object.entries(check.counts) as [status, count] (status)}
					{#if count > 0}<span>{importLabel(status as NewsletterImportStatus)}: {number(count)}</span>{/if}
				{/each}
			</p>
			<ul class="rows">
				{#each check.rows.filter((r) => r.status !== 'new') as r (r.line)}
					<li><span class="muted">{r.line}</span> {r.email ?? '—'} · {importLabel(r.status)}</li>
				{/each}
			</ul>
		{/if}
	</div>
	{#snippet footer()}
		<Button size="sm" variant="secondary" onclick={() => (importOpen = false)}>{m['common.cancel']()}</Button>
		{#if check}
			<Button size="sm" disabled={importing || check.counts.new === 0} onclick={() => runImport(false)}>{m['newsletter.import_run']({ count: number(check.counts.new) })}</Button>
		{:else}
			<Button size="sm" disabled={importing || !importText.trim() || !importSource.trim()} onclick={() => runImport(true)}>{m['newsletter.import_check']()}</Button>
		{/if}
	{/snippet}
</Sheet>

<style>
	.locale.missing {
		background: var(--badge-warning);
		color: var(--badge-warning-text);
	}
	.import {
		display: grid;
		gap: var(--space-1);
	}
	.import textarea {
		width: 100%;
	}
	.counts {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-1);
		margin: 0;
	}
	.rows {
		margin: 0;
		padding: 0;
		list-style: none;
		max-height: 16rem;
		overflow: auto;
	}
</style>
