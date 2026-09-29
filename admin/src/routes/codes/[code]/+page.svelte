<script lang="ts">
	import { page } from '$app/state';
	import { ActionBar, Badge, Button, DataTable, RecordInspector, Section, Skeleton, type Column } from '@scutoons/ui';
	import type { AdminCodeDetail, CodeEvent } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { errorText, minute, number } from '$lib/format.ts';
	import { eventLabel, inspectorLabels, tableLabels } from '$lib/labels.ts';
	import { qrUrl } from '$lib/qr/config.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PageHeader from '$lib/components/page-header.svelte';
	import CodeStatus from '$lib/components/code-status.svelte';
	import RevokeForm from '$lib/components/revoke-form.svelte';
	import CertificatePanel from '$lib/components/certificate-panel.svelte';

	const code = $derived(page.params.code ?? '');

	let detail = $state<AdminCodeDetail | null>(null);
	let events = $state<CodeEvent[]>([]);
	let error = $state<string | null>(null);

	$effect(() => {
		const current = code;
		detail = null;
		error = null;
		Promise.all([admin.getCode(current), admin.codeEvents(current, 5000)])
			.then(([d, e]) => {
				detail = d;
				events = e.items;
			})
			.catch((e) => (error = errorText(e)));
	});

	const eventRows = $derived(events.map((e, i) => ({ ...e, key: String(i) })));

	const eventColumns = $derived<Column[]>([
		{ id: 'created_at', label: m['events.col_when']() },
		{ id: 'event', label: m['events.col_event']() },
		{ id: 'device', label: m['events.col_device']() },
		{ id: 'account', label: m['events.col_account']() },
		{ id: 'toon', label: m['events.col_toon'](), secondary: true }
	]);
</script>

{#if error}
	<PageHeader type={m['codes.type_label']()} title={code} />
	<p class="error-box" role="alert">{error}</p>
{:else if !detail}
	<PageHeader type={m['codes.type_label']()} title={code} />
	<Skeleton lines={10} label={m['shell.loading']()} />
{:else}
	{@const c = detail.code}
	<PageHeader
		type={m['codes.type_label']()}
		title={c.code}
		meta={`${c.type} · ${detail.edition ? `${detail.edition.name} #${c.edition_number ?? '—'}` : m['common.no_edition']()}`}
	>
		<CodeStatus claimed={c.claimed} revoked={detail.revoked} expired={detail.expired} signed={detail.signed} />
	</PageHeader>

	<ActionBar label={m['shell.page_actions']()} moreLabel={m['shell.more']()}>
		{#snippet start()}
			<a href="/codes">← {m['codes.back_to_list']()}</a>
		{/snippet}
		{#snippet secondary()}
			<Button size="sm" variant="secondary" href="/events?code={encodeURIComponent(c.code)}">{m['codes.in_events_log']()}</Button>
		{/snippet}
		{#snippet end()}
			<Button size="sm" type="submit" form="revoke-form">{detail!.revoked ? m['codes.unrevoke']() : m['codes.revoke']()}</Button>
		{/snippet}
	</ActionBar>

	<div class="grid-2">
		<Section title={m['codes.section_facts']()} hint={m['codes.section_facts_hint']()}>
			<dl class="facts">
				<dt>{m['codes.qr_url']()}</dt>
				<dd>{qrUrl(c.code)}</dd>
				<dt>{m['codes.redirect']()}</dt>
				<dd><a href={c.redirect_to} target="_blank" rel="noreferrer">{c.redirect_to}</a></dd>
				<dt>{m['codes.created']()}</dt>
				<dd>{minute(c.created_at)}</dd>
				<dt>{m['codes.col_claimed']()}</dt>
				<dd>{c.claimed ? minute(c.claimed_at) : '—'}</dd>
				<dt>{m['codes.expires']()}</dt>
				<dd>{c.expires_at ? minute(c.expires_at) : m['codes.no_expiry']()}</dd>
				<dt>{m['common.edition']()}</dt>
				<dd>
					{#if detail.edition}
						{detail.edition.name} ({detail.edition.id}) · {m['codes.number_of']({ number: detail.edition.number ?? '—', count: detail.edition.count ?? '—' })}
					{:else}—{/if}
				</dd>
				<dt>{m['codes.items']()}</dt>
				<dd>
					{#each detail.items as item, i (i)}
						<Badge>{item.item_id}{item.variant ? ` · ${item.variant}` : ''} × {item.qty}</Badge>
					{:else}—{/each}
				</dd>
				{#if detail.options.length}
					<dt>{m['codes.options']()}</dt>
					<dd>
						{#each detail.options as option (option.item_id)}
							<Badge>{option.item_id}{option.variants ? `: ${option.variants.join(', ')}` : ''}</Badge>
						{/each}
					</dd>
				{/if}
			</dl>
		</Section>

		<Section title={m['codes.section_status']()} hint={m['codes.section_status_hint']()}>
			<RevokeForm
				formId="revoke-form"
				inlineSubmit={false}
				code={c.code}
				revoked={detail.revoked}
				reason={c.revoked_reason}
				onchanged={(state) => {
					if (!detail) return;
					detail.code.revoked_at = state.revoked_at;
					detail.code.revoked_reason = state.revoked_reason;
					detail.revoked = !!state.revoked_at;
					admin.codeEvents(c.code, 5000).then((e) => (events = e.items));
				}}
			/>
			{#if c.revoked_at}
				<p class="muted">{m['codes.revoked_at']({ when: minute(c.revoked_at) })}</p>
			{/if}
		</Section>
	</div>

	<Section title={m['codes.section_activity']()} hint={m['codes.section_activity_hint']()}>
		<ul class="stats">
			<li><strong>{number(detail.stats.scans)}</strong> {m['codes.col_scans']()}</li>
			<li><strong>{number(detail.stats.devices)}</strong> {m['codes.col_devices']()}</li>
			<li><strong>{number(detail.stats.accesses)}</strong> {m['codes.col_unlocks']()}</li>
			<li><strong>{number(detail.stats.logins)}</strong> {m['codes.col_logins']()}</li>
			<li><strong>{number(detail.stats.refused)}</strong> {m['codes.col_refused']()}</li>
		</ul>
		<DataTable
			columns={eventColumns}
			rows={eventRows}
			rowKey={(e) => e.key}
			emptyLabel={m['codes.no_events']()}
			caption={m['codes.timeline_caption']({ count: events.length })}
			labels={tableLabels()}
		>
			{#snippet cell(e, column)}
				{#if column.id === 'event'}{eventLabel(e.event)}
				{:else if column.id === 'account'}{e.account ? m['common.yes']() : '—'}
				{:else if column.id === 'created_at'}{e.created_at}
				{:else}{e[column.id as 'device' | 'toon'] ?? '—'}{/if}
			{/snippet}
		</DataTable>
	</Section>

	<Section title={m['codes.section_certificate']()} hint={m['codes.section_certificate_hint']()}>
		<CertificatePanel code={c.code} type={c.type} />
	</Section>

	<Section title={m['codes.section_raw']()} hint={m['codes.section_raw_hint']()}>
		<RecordInspector record={detail} labels={inspectorLabels()} />
	</Section>
{/if}

<style>
	.stats {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05) var(--space-15);
		margin: 0 0 var(--space-1);
		padding: 0;
		list-style: none;
	}
	.stats strong {
		font-size: 1.3em;
	}
</style>
