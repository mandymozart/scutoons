<script lang="ts">
	import { infoBar } from '$lib/info.ts';
	import PageHelp from '$lib/components/page-help.svelte';
	import EmailsNav from '$lib/components/emails-nav.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { ActionBar, Badge, Button, Field, Section, Segmented, Skeleton } from '@scutoons/ui';
	import type { AdminEmailPreview, AdminEmailRecipients, AdminEmailTemplates } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { errorText, minute } from '$lib/format.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import PageHeader from '$lib/components/page-header.svelte';

	/**
	 * The email module's templates (GET /api/admin/emails/templates), each rendered with its sample values in the
	 * chosen language (?locale=de|en, …/preview), and the admin notification recipients (GET/PUT …/emails/recipients).
	 */
	const toaster = getToaster();

	let templates = $state<AdminEmailTemplates | null>(null);
	/** template id → its preview in the current locale */
	let previews = $state<Record<string, AdminEmailPreview>>({});
	let recipients = $state<AdminEmailRecipients | null>(null);
	/** the recipients textarea: one address per line */
	let draft = $state('');
	let error = $state<string | null>(null);
	let saving = $state(false);

	const locale = $derived(templates?.locales.includes(page.url.searchParams.get('locale') ?? '') ? (page.url.searchParams.get('locale') as string) : getLocale());

	Promise.all([admin.emailTemplates(), admin.emailRecipients()])
		.then(([t, r]) => {
			templates = t;
			apply(r);
		})
		.catch((e) => (error = errorText(e)));

	$effect(() => {
		if (!templates) return;
		const wanted = locale;
		Promise.all(templates.items.map((t) => admin.emailPreview(t.id, wanted)))
			.then((list) => (previews = Object.fromEntries(list.map((p) => [p.id, p]))))
			.catch((e) => (error = errorText(e)));
	});

	function apply(next: AdminEmailRecipients) {
		recipients = next;
		draft = next.items.join('\n');
	}

	const EMAIL = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
	const lines = $derived(draft.split(/[\n,;]+/).map((l) => l.trim().toLowerCase()).filter(Boolean));
	const invalidLine = $derived(lines.find((l) => !EMAIL.test(l)) ?? null);
	const tooMany = $derived(recipients !== null && new Set(lines).size > recipients.max);
	const changed = $derived(recipients !== null && [...new Set(lines)].join('\n') !== recipients.items.join('\n'));

	async function save(items: string[] | null) {
		if (items !== null && (!changed || invalidLine || tooMany)) {
			if (!changed) toaster.addToast({ message: m['emails.nothing_changed'](), type: 'info' });
			return;
		}
		saving = true;
		try {
			apply(await admin.saveEmailRecipients(items));
			toaster.addToast({ message: items === null ? m['emails.reset_done']() : m['emails.saved'](), type: 'success' });
		} catch (e) {
			toaster.addToast({ message: errorText(e), type: 'error' });
		} finally {
			saving = false;
		}
	}

	const setLocale = (next: string) => {
		const url = new URL(page.url);
		url.searchParams.set('locale', next);
		goto(url, { keepFocus: true, noScroll: true, replaceState: true });
	};
	const audience = (a: string) => (a === 'admin' ? m['emails.audience_admin']() : a === 'member' ? m['emails.audience_member']() : a);
</script>

<PageHeader type={m['nav.group_system']()} title={m['emails.title']()} meta={m['emails.meta_templates']()} />
<EmailsNav current="templates" />

<ActionBar label={m['shell.page_actions']()} moreLabel={m['shell.more']()} {...infoBar()} infoTitle={m['emails.title']()}>
	{#snippet info()}<PageHelp page="emails" />{/snippet}
	{#snippet start()}
		<span class="bar-filter">
			<span>{m['emails.language']()}</span>
			<Segmented label={m['emails.language']()} value={locale} onchange={setLocale} options={(templates?.locales ?? ['de', 'en']).map((l) => ({ value: l, label: l.toUpperCase() }))} />
		</span>
		{#if changed}<span class="muted">{m['emails.unsaved']()}</span>{/if}
	{/snippet}
	{#snippet end()}
		{#if recipients?.stored}
			<Button size="sm" variant="secondary" onclick={() => save(null)} disabled={saving}>{m['emails.use_default']()}</Button>
		{/if}
		<Button size="sm" onclick={() => save([...new Set(lines)])} disabled={!recipients || saving || !changed || !!invalidLine || tooMany}>
			{saving ? m['emails.saving']() : m['emails.save']()}
		</Button>
	{/snippet}
</ActionBar>

{#if error}<p class="error-box" role="alert">{error}</p>{/if}

{#if !templates || !recipients}
	{#if !error}<Skeleton lines={8} label={m['shell.loading']()} />{/if}
{:else}
	<p class="muted transport">{templates.transport === 'log' ? m['emails.transport_log']() : m['emails.transport']({ transport: templates.transport })}</p>

	{#each templates.items as t (t.id)}
		{@const preview = previews[t.id]}
		<Section title={t.id}>
			<dl class="facts">
				<dt>{m['emails.audience']()}</dt>
				<dd><Badge tone={t.audience === 'admin' ? 'info' : 'neutral'}>{audience(t.audience)}</Badge></dd>
				<dt>{m['emails.description']()}</dt>
				<dd>{t.description}</dd>
				<dt>{m['emails.vars']()}</dt>
				<dd class="vars">
					{#each Object.entries(t.vars) as [name, type] (name)}<code>{name}: {type}</code>{/each}
				</dd>
				<dt>{m['emails.subject']()}</dt>
				<dd class="subject">{preview?.subject ?? '…'}</dd>
			</dl>
			{#if preview}
				<div class="preview grid-2">
					<figure>
						<figcaption class="sc-label">{m['emails.html']()}</figcaption>
						<!-- the rendered mail, isolated: no scripts, no same-origin access -->
						<iframe title={m['emails.preview_title']({ id: t.id })} sandbox="" srcdoc={preview.html}></iframe>
					</figure>
					<figure>
						<figcaption class="sc-label">{m['emails.text']()}</figcaption>
						<pre>{preview.text}</pre>
					</figure>
				</div>
			{/if}
		</Section>
	{/each}

	<Section title={m['emails.recipients']()}>
		<div class="recipients">
			<Field
				label={m['emails.addresses']()}
				hint={m['emails.recipients_hint']({ max: recipients.max })}
				error={invalidLine ? m['emails.recipients_invalid']({ value: invalidLine }) : tooMany ? m['emails.recipients_too_many']({ max: recipients.max }) : null}
			>
				{#snippet children({ id, describedby, invalid })}
					<textarea {id} rows="4" spellcheck="false" aria-describedby={describedby} aria-invalid={invalid} bind:value={draft}></textarea>
				{/snippet}
			</Field>
			<p class="muted state">
				<span>{m['emails.recipients_default']({ list: recipients.default.length ? recipients.default.join(', ') : m['emails.recipients_nobody']() })}</span>
				<span>
					{recipients.stored && recipients.updated_at
						? m['emails.last_change']({ at: minute(recipients.updated_at), by: (recipients.updated_by ?? '—').replace(/^admin:/, '') })
						: m['emails.state_default']()}
				</span>
			</p>
		</div>
	</Section>
{/if}

<style>
	.transport {
		margin: 0 0 var(--space-15);
	}
	.vars {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05);
	}
	.subject {
		font-weight: var(--font-weight-regular);
	}
	.preview {
		margin-top: var(--space-15);
	}
	figure {
		margin: 0;
		min-width: 0;
	}
	figcaption {
		margin-bottom: var(--space-05);
	}
	iframe {
		display: block;
		box-sizing: border-box;
		width: 100%;
		height: 32rem;
		border: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
	}
	pre {
		box-sizing: border-box;
		margin: 0;
		height: 32rem;
		overflow: auto;
		padding: var(--space-1);
		border: var(--border-width-regular) solid var(--ink);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
		font: inherit;
	}
	.recipients {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		max-width: 44rem;
	}
	.recipients textarea {
		width: 100%;
		box-sizing: border-box;
	}
	.state {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05) var(--space-15);
		margin: 0;
		font-size: var(--type-caption-size);
	}
</style>
