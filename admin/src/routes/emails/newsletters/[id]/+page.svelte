<script lang="ts">
	import { tick } from 'svelte';
	import EmailsNav from '$lib/components/emails-nav.svelte';
	import NewsletterGift from '$lib/components/newsletter-gift.svelte';
	import NewsletterPreview from '$lib/components/newsletter-preview.svelte';
	import NewsletterRecipients from '$lib/components/newsletter-recipients.svelte';
	import PageHeader from '$lib/components/page-header.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { ActionBar, Badge, Button, Field, Section, Segmented, Skeleton, confirmDialog } from '@scutoons/ui';
	import type { Newsletter, NewsletterAudience, NewsletterLocale } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { errorText, number } from '$lib/format.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { audienceLabel, newsletterError, newsletterStatus, statusTone } from '$lib/newsletter.ts';
	import { insertButton, insertInline, insertParagraph, type Inserted } from '$lib/newsletter-text.ts';
	import { AssetPickerSheet, type PickedAsset } from '@scutoons/media';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * One newsletter (agents/NEWSLETTER.md "Writing a newsletter"): one column on the creator's layout model (#98,
	 * DOCKING.md) – title, who gets it, the gift, then the text (DE | EN, subject, a tool strip for pictures, buttons and
	 * placeholders); the live preview (unsaved text, the frame of every platform mail) is a dock panel beside it.
	 * Test to me, start / pause / resume, the recipients. A draft changes freely; a paused one only its subject and text;
	 * a sending or finished one not at all.
	 */
	const toaster = getToaster();
	const id = $derived(Number(page.params.id));

	let item = $state<Newsletter | null>(null);
	let error = $state<string | null>(null);
	let audiences = $state<NewsletterAudience[]>([]);
	let selection = $state<{ count: number; missing_locale: number; locales: Record<string, number> } | null>(null);
	let locale = $state<NewsletterLocale>('de');
	let busy = $state(false);
	let draft = $state({ title: '', audiences: [] as string[], subject_de: '', subject_en: '', body_de: '', body_en: '', gift_campaign_id: null as string | null });
	let previewHtml = $state('');
	// the preview panel: open from the start on a wide screen (on a phone it would cover the page)
	let previewOpen = $state(typeof window !== 'undefined' && window.innerWidth >= 768);
	// pictures (owner 2026-10-09): the asset manager's picker puts `![](path)` as its own paragraph at the cursor
	let picking = $state(false);
	let bodyArea = $state<HTMLTextAreaElement | null>(null);
	// the text of the language shown; the tool strip's inserts (lib/newsletter-text.ts) write it and put the cursor
	const bodyKey = $derived(locale === 'de' ? 'body_de' : 'body_en');
	function put(done: Inserted, focus = true) {
		draft[bodyKey] = done.text;
		void tick().then(() => {
			if (focus) bodyArea?.focus();
			bodyArea?.setSelectionRange(done.start, done.end);
		});
	}
	const cursor = () => bodyArea?.selectionStart ?? draft[bodyKey].length;
	function insertPicture(picked: PickedAsset) {
		put(insertParagraph(draft[bodyKey], cursor(), `![](${picked.path})`), false);
		// many pictures exist only as WebP: Outlook (desktop) shows none
		if (/\.webp$/i.test(picked.path)) toaster.addToast({ message: m['newsletter.picture_webp'](), type: 'warning' });
	}
	// the placeholders the renderer knows (NewsletterRenderer: {name}, {name|fallback}; the {signin} / {gift} lines –
	// #65, #66 – are paragraphs of their own), picked from a list (owner 2026-10-09)
	const LINES = ['{signin}', '{gift}'];
	const variables = $derived([
		{ value: '{name}', label: m['newsletter.variable_name']() },
		{ value: locale === 'de' ? '{name|du}' : '{name|there}', label: m['newsletter.variable_name_or']({ word: locale === 'de' ? 'du' : 'there' }) },
		{ value: '{signin}', label: m['newsletter.variable_signin']() },
		{ value: '{gift}', label: m['newsletter.variable_gift']() }
	]);
	function insertVariable(value: string) {
		const start = cursor();
		const end = bodyArea?.selectionEnd ?? start;
		put(LINES.includes(value) ? insertParagraph(draft[bodyKey], start, value, end) : insertInline(draft[bodyKey], start, end, value));
	}
	let previewSubject = $state('');

	const fields = ['title', 'audiences', 'subject_de', 'subject_en', 'body_de', 'body_en', 'gift_campaign_id'] as const;
	const dirty = $derived(!!item && fields.some((f) => JSON.stringify(draft[f]) !== JSON.stringify(item![f])));
	const editable = $derived(item?.status === 'draft');
	const textEditable = $derived(item?.status === 'draft' || item?.status === 'paused');

	function apply(next: Newsletter) {
		item = next;
		draft = {
			title: next.title,
			audiences: [...next.audiences],
			subject_de: next.subject_de,
			subject_en: next.subject_en,
			body_de: next.body_de,
			body_en: next.body_en,
			gift_campaign_id: next.gift_campaign_id
		};
	}

	$effect(() => {
		const current = id;
		error = null;
		admin
			.newsletter(current)
			.then((r) => apply(r.item))
			.catch((e) => (error = errorText(e)));
	});

	// the audiences with their counts, and what the current choice reaches together
	$effect(() => {
		const chosen = [...draft.audiences];
		if (!item) return;
		admin
			.newsletterAudiences(chosen.length ? chosen : undefined)
			.then((r) => {
				audiences = r.items;
				selection = r.selected ?? { count: 0, missing_locale: 0, locales: {} };
			})
			.catch(() => {});
	});

	// the live preview of the current language's unsaved text
	$effect(() => {
		if (!item) return;
		const subject = locale === 'de' ? draft.subject_de : draft.subject_en;
		const body = locale === 'de' ? draft.body_de : draft.body_en;
		const current = locale;
		const timer = setTimeout(() => {
			admin
				.previewNewsletter(item!.id, current, { subject, body })
				.then((r) => {
					previewHtml = r.html;
					previewSubject = r.subject;
				})
				.catch(() => {});
		}, 400);
		return () => clearTimeout(timer);
	});

	function toggleAudience(audienceId: string, on: boolean) {
		draft.audiences = on ? [...draft.audiences, audienceId] : draft.audiences.filter((a) => a !== audienceId);
	}

	async function save(): Promise<boolean> {
		if (!item || !dirty) return true;
		busy = true;
		try {
			const patch = editable ? { ...draft } : { subject_de: draft.subject_de, subject_en: draft.subject_en, body_de: draft.body_de, body_en: draft.body_en };
			apply((await admin.saveNewsletter(item.id, patch)).item);
			toaster.addToast({ message: m['newsletter.saved'](), type: 'success' });
			return true;
		} catch (e) {
			toaster.addToast({ message: newsletterError(e), type: 'error' });
			return false;
		} finally {
			busy = false;
		}
	}

	async function test() {
		if (!item || !(await save())) return;
		busy = true;
		try {
			const r = await admin.testNewsletter(item.id, locale);
			toaster.addToast({ message: m['newsletter.test_sent']({ to: r.to }), type: 'success' });
		} catch (e) {
			toaster.addToast({ message: newsletterError(e), type: 'error' });
		} finally {
			busy = false;
		}
	}

	async function action(name: 'start' | 'pause' | 'resume') {
		if (!item) return;
		if (name !== 'pause' && !(await save())) return;
		if (name === 'start') {
			const sending = await admin.newsletterState().catch(() => null);
			const ok = await confirmDialog({
				title: m['newsletter.start_title']({ count: number(selection?.count ?? 0) }),
				message: m['newsletter.start_message']({ limit: sending && sending.per_hour > 0 ? number(sending.per_hour) : '∞' }),
				confirmLabel: m['newsletter.start']()
			});
			if (!ok) return;
		}
		busy = true;
		try {
			apply((await admin.newsletterAction(item.id, name)).item);
			if (name === 'start') toaster.addToast({ message: m['newsletter.started'](), type: 'success' });
		} catch (e) {
			toaster.addToast({ message: newsletterError(e), type: 'error' });
		} finally {
			busy = false;
		}
	}

	async function remove() {
		if (!item) return;
		if (!(await confirmDialog({ title: m['newsletter.delete_title'](), message: m['newsletter.delete_message'](), confirmLabel: m['newsletter.delete']() }))) return;
		try {
			await admin.deleteNewsletter(item.id);
			goto('/emails/newsletters');
		} catch (e) {
			toaster.addToast({ message: newsletterError(e), type: 'error' });
		}
	}

	const missingFor = (a: NewsletterAudience) =>
		a.source ? `/emails/contacts?locale=none&source=${encodeURIComponent(a.source)}` : '/emails/contacts?locale=none';
</script>

<PageHeader
	type={m['newsletter.title']()}
	title={item?.title ?? '…'}
	meta={item ? (item.progress.total > 0 ? m['newsletter.progress']({ sent: number(item.progress.sent), total: number(item.progress.total) }) : undefined) : undefined}
/>
<EmailsNav current="newsletters" />

{#snippet barStart()}
	<a href="/emails/newsletters">{m['newsletter.back']()}</a>
	{#if item}<Badge tone={statusTone(item.status)}>{newsletterStatus(item.status)}</Badge>{/if}
	{#if dirty}<span class="muted">{m['newsletter.unsaved']()}</span>{/if}
{/snippet}

<ActionBar start={barStart} label={m['shell.page_actions']()} moreLabel={m['shell.more']()}>
	{#snippet secondary()}
		{#if item && !previewOpen}
			<Button size="sm" variant="secondary" onclick={() => (previewOpen = true)}>{m['newsletter.preview']()}</Button>
		{/if}
		{#if item && (item.status === 'draft' || item.status === 'done')}
			<Button size="sm" variant="secondary" onclick={remove}>{m['newsletter.delete']()}</Button>
		{/if}
	{/snippet}
	{#snippet end()}
		{#if item}
			{#if textEditable}
				<Button size="sm" variant="secondary" disabled={busy || !dirty} onclick={save}>{m['newsletter.save']()}</Button>
				<Button size="sm" variant="secondary" disabled={busy} onclick={test}>{m['newsletter.test']()}</Button>
			{/if}
			{#if item.status === 'draft'}
				<Button size="sm" disabled={busy || !selection || selection.count === 0} onclick={() => action('start')}>{m['newsletter.start']()}</Button>
			{:else if item.status === 'sending'}
				<Button size="sm" disabled={busy} onclick={() => action('pause')}>{m['newsletter.pause']()}</Button>
			{:else if item.status === 'paused'}
				<Button size="sm" disabled={busy} onclick={() => action('resume')}>{m['newsletter.resume']()}</Button>
			{/if}
		{/if}
	{/snippet}
</ActionBar>

{#if error}<p class="error-box" role="alert">{error}</p>{/if}

{#if !item}
	{#if !error}<Skeleton lines={8} label={m['shell.loading']()} />{/if}
{:else}
<div class="editor">
	{#if item.last_error}<p class="error-box" role="alert">{m['newsletter.last_error']({ error: item.last_error })}</p>{/if}
	{#if item.status === 'sending'}<p class="muted">{m['newsletter.locked_sending']()}</p>{/if}
	{#if item.status === 'done'}<p class="muted">{m['newsletter.locked_done']()}</p>{/if}

	{#if editable}
		<Field label={m['newsletter.new_title']()} hint={m['newsletter.new_title_hint']()}>
			{#snippet children({ id: fieldId, describedby })}
				<input id={fieldId} aria-describedby={describedby} maxlength="200" bind:value={draft.title} />
			{/snippet}
		</Field>
	{/if}

	<Section title={m['newsletter.audiences']()}>
		<p class="muted hint">{m['newsletter.audiences_hint']()}</p>
		<fieldset class="audiences" disabled={!editable} aria-label={m['newsletter.audiences']()}>
			{#each audiences as a (a.id)}
				<label class="audience">
					<input type="checkbox" checked={draft.audiences.includes(a.id)} onchange={(e) => toggleAudience(a.id, e.currentTarget.checked)} />
					<span>{audienceLabel(a)}</span>
					<span class="nums muted">{number(a.count)}</span>
					{#if a.missing_locale > 0}
						<a href={missingFor(a)}>{m['newsletter.audience_missing']({ count: number(a.missing_locale) })}</a>
					{/if}
				</label>
			{/each}
		</fieldset>
		{#if selection && editable}
			<p class="total">
				{m['newsletter.recipients_total']({ count: number(selection.count) })}
				{#each Object.entries(selection.locales) as [code, count] (code)}
					<span class="muted"> · {code === 'none' ? m['newsletter.language_none']() : code} {number(count)}</span>
				{/each}
			</p>
		{/if}
	</Section>

	<NewsletterGift newsletterId={item.id} value={draft.gift_campaign_id} audiences={draft.audiences} {editable} onchange={(next) => (draft.gift_campaign_id = next)} />

	<Section title={m['newsletter.text']()}>
		<div class="text">
			<Segmented
				label={m['newsletter.language']()}
				options={[
					{ value: 'de', label: 'DE' },
					{ value: 'en', label: 'EN' }
				]}
				value={locale}
				onchange={(v) => (locale = v)}
			/>
			{#if locale === 'de'}
				<Field label={m['newsletter.subject']()}>
					{#snippet children({ id: fieldId })}<input id={fieldId} maxlength="200" disabled={!textEditable} bind:value={draft.subject_de} />{/snippet}
				</Field>
			{:else}
				<Field label={m['newsletter.subject']()}>
					{#snippet children({ id: fieldId })}<input id={fieldId} maxlength="200" disabled={!textEditable} bind:value={draft.subject_en} />{/snippet}
				</Field>
			{/if}
			{#if textEditable}
				<div class="tools" role="toolbar" aria-label={m['newsletter.tools']()}>
					<Button size="sm" variant="secondary" onclick={() => (picking = true)}>{m['newsletter.picture_add']()}</Button>
					<Button size="sm" variant="secondary" onclick={() => put(insertButton(draft[bodyKey], cursor(), m['newsletter.button_text']()))}>{m['newsletter.button_add']()}</Button>
					<select
						aria-label={m['newsletter.variable_add']()}
						value=""
						onchange={(e) => {
							const value = e.currentTarget.value;
							e.currentTarget.value = '';
							if (value) insertVariable(value);
						}}
					>
						<option value="" disabled>{m['newsletter.variable_add']()}</option>
						{#each variables as v (v.value)}
							<option value={v.value}>{v.value} – {v.label}</option>
						{/each}
					</select>
				</div>
			{/if}
			<Field label={m['newsletter.body']()} hint={m['newsletter.body_hint']({ example: locale === 'de' ? '{name|du}' : '{name|there}' })}>
				{#snippet children({ id: fieldId, describedby })}
					{#if locale === 'de'}
						<textarea bind:this={bodyArea} id={fieldId} aria-describedby={describedby} rows="18" disabled={!textEditable} bind:value={draft.body_de}></textarea>
					{:else}
						<textarea bind:this={bodyArea} id={fieldId} aria-describedby={describedby} rows="18" disabled={!textEditable} bind:value={draft.body_en}></textarea>
					{/if}
				{/snippet}
			</Field>
		</div>
	</Section>

	<NewsletterPreview open={previewOpen} subject={previewSubject} html={previewHtml} onclose={() => (previewOpen = false)} />

	{#if item.status !== 'draft'}<NewsletterRecipients {item} />{/if}
</div>
{/if}

<AssetPickerSheet bind:open={picking} kind="image" place="newsletter/" title={m['newsletter.picture_add']()} onpick={insertPicture} />

<style>
	.audiences {
		display: grid;
		gap: var(--space-05);
		margin: 0;
		padding: 0;
		border: 0;
	}
	.hint {
		margin: 0 0 var(--space-1);
	}
	.audience {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-1);
	}
	.total {
		margin: var(--space-1) 0 0;
	}
	/* one readable column; the preview is a dock panel beside it (#98) */
	.editor {
		display: grid;
		/* never wider than the main – a long option text must not widen the column */
		grid-template-columns: minmax(0, 1fr);
		gap: var(--space-2);
		max-width: 40rem;
	}
	.text {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		gap: var(--space-1);
	}
	/* picture, button, placeholders – right above the text they go into */
	.tools {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05);
	}
	.tools select {
		flex: 1 1 12rem;
		min-width: 0;
	}
	.text textarea {
		width: 100%;
		min-width: 0;
		box-sizing: border-box;
		resize: vertical;
		font-family: var(--font-mono, monospace);
	}
</style>
