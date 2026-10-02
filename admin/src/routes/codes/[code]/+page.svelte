<script lang="ts">
	import { infoBar } from '$lib/info.ts';
	import PageHelp from '$lib/components/page-help.svelte';
	import { page } from '$app/state';
	import { ActionBar, Badge, Button, DataTable, Field, RecordInspector, Section, Sheet, Skeleton, type Column } from '@scutoons/ui';
	import type { AdminCodeDetail, CodeEvent, OwnershipTransfer } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { errorText, minute, number } from '$lib/format.ts';
	import { eventLabel, inspectorLabels, tableLabels } from '$lib/labels.ts';
	import { qrUrl } from '$lib/qr/config.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PageHeader from '$lib/components/page-header.svelte';
	import CodeStatus from '$lib/components/code-status.svelte';
	import RevokeForm from '$lib/components/revoke-form.svelte';
	import CertificatePanel from '$lib/components/certificate-panel.svelte';
	import OwnershipHistory from '$lib/components/ownership-history.svelte';
	import { choiceLabel, itemChoices, type ItemChoice } from '$lib/claims.svelte.ts';
	import { getToaster } from '$lib/toasts.ts';

	const code = $derived(page.params.code ?? '');

	let detail = $state<AdminCodeDetail | null>(null);
	let events = $state<CodeEvent[]>([]);
	let error = $state<string | null>(null);
	let history = $state<OwnershipTransfer[]>([]);
	let historyError = $state<string | null>(null);

	$effect(() => {
		const current = code;
		detail = null;
		error = null;
		history = [];
		historyError = null;
		transferTo = '';
		transferNote = '';
		choices = null;
		choice = '';
		Promise.all([admin.getCode(current), admin.codeEvents(current, 5000)])
			.then(([d, e]) => {
				detail = d;
				events = e.items;
			})
			.catch((e) => (error = errorText(e)));
		// the ownership history has its own call: the code page still loads when it fails
		admin
			.codeHistory(current)
			.then((h) => (history = h.items))
			.catch((e) => (historyError = errorText(e)));
	});

	// ── transfer ownership (POST /admin/codes/{code}/transfer): a support case outside a claim ──
	const toaster = getToaster();
	let transferTo = $state('');
	let transferNote = $state('');
	let choices = $state<ItemChoice[] | null>(null);
	let choice = $state('');
	let transferConfirm = $state(false);
	let transferring = $state(false);
	const canTransfer = $derived(!!transferTo.trim() && !!transferNote.trim() && (!choices || choice !== ''));

	function askTransfer(event: SubmitEvent) {
		event.preventDefault();
		if (canTransfer) transferConfirm = true;
	}

	async function transfer() {
		if (!detail) return;
		const current = detail.code.code;
		const picked = choices && choice !== '' ? choices[Number(choice)] : null;
		transferring = true;
		try {
			const result = await admin.transferCode(current, {
				to: transferTo.trim(),
				note: transferNote.trim(),
				...(picked ? { item_id: picked.item_id, ...(picked.variant ? { variant: picked.variant } : {}) } : {})
			});
			transferConfirm = false;
			history = result.history;
			transferTo = '';
			transferNote = '';
			choices = null;
			choice = '';
			toaster.addToast({ message: m['claims.code_transferred']({ code: current, email: result.to.email ?? result.to.id }), type: 'success' });
			const [d, e] = await Promise.all([admin.getCode(current), admin.codeEvents(current, 5000)]);
			detail = d;
			events = e.items;
		} catch (e) {
			transferConfirm = false;
			const options = itemChoices(e);
			if (options) {
				choices = options;
				choice = '';
				toaster.addToast({ message: m['claims.choose_item_needed'](), type: 'error' });
			} else {
				toaster.addToast({ message: errorText(e), type: 'error' });
			}
		} finally {
			transferring = false;
		}
	}

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

	<ActionBar label={m['shell.page_actions']()} moreLabel={m['shell.more']()} {...infoBar()} infoTitle={code}>
		{#snippet info()}<PageHelp page="code" />{/snippet}
		{#snippet start()}
			<a href="/codes">{m['codes.back_to_list']()}</a>
		{/snippet}
		{#snippet secondary()}
			<Button size="sm" variant="secondary" href="/events?code={encodeURIComponent(c.code)}">{m['codes.in_events_log']()}</Button>
			<Button size="sm" variant="secondary" href="/claims?code={encodeURIComponent(c.code)}">{m['claims.for_this_code']()}</Button>
		{/snippet}
		{#snippet end()}
			<Button size="sm" type="submit" form="revoke-form">{detail!.revoked ? m['codes.unrevoke']() : m['codes.revoke']()}</Button>
		{/snippet}
	</ActionBar>

	<div class="grid-2">
		<Section title={m['codes.section_facts']()}>
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

		<Section title={m['codes.section_status']()}>
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

	<Section title={m['codes.section_activity']()}>
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

	<Section title={m['claims.section_history']()} hint={m['claims.code_history_hint']()}>
		{#snippet actions()}
			<a href="/claims?code={encodeURIComponent(c.code)}">{m['claims.for_this_code']()}</a>
		{/snippet}
		{#if historyError}<p class="error-box" role="alert">{historyError}</p>{/if}
		<OwnershipHistory items={history} />
	</Section>

	<Section title={m['claims.section_transfer_code']()} hint={m['claims.section_transfer_code_hint']()}>
		<form id="transfer-code-form" class="transfer" onsubmit={askTransfer} data-testid="transfer-code-form">
			<Field label={m['claims.transfer_to']()} hint={m['claims.transfer_to_hint']()} required>
				{#snippet children({ id, describedby })}
					<input {id} aria-describedby={describedby} maxlength="255" autocomplete="off" bind:value={transferTo} />
				{/snippet}
			</Field>
			<Field label={m['claims.note']()} hint={m['claims.transfer_code_note_hint']()} required>
				{#snippet children({ id, describedby })}
					<textarea {id} aria-describedby={describedby} rows="3" maxlength="2000" bind:value={transferNote}></textarea>
				{/snippet}
			</Field>
			{#if choices}
				<Field label={m['claims.choose_item']()} hint={m['claims.choose_item_hint']()} required>
					{#snippet children({ id, describedby })}
						<select {id} aria-describedby={describedby} bind:value={choice}>
							<option value="">{m['claims.choose_item_placeholder']()}</option>
							{#each choices ?? [] as option, i (i)}
								<option value={String(i)}>{choiceLabel(option)}</option>
							{/each}
						</select>
					{/snippet}
				</Field>
			{/if}
			<div><Button size="sm" type="submit" disabled={!canTransfer || transferring}>{m['claims.transfer_code']()}</Button></div>
		</form>
	</Section>

	<Sheet bind:open={transferConfirm} title={m['claims.transfer_code']()} closeLabel={m['common.close']()}>
		<p>{m['claims.confirm_transfer_code']({ code: c.code, to: transferTo.trim() })}</p>
		{#snippet footer()}
			<Button size="sm" variant="secondary" onclick={() => (transferConfirm = false)}>{m['common.cancel']()}</Button>
			<Button size="sm" disabled={transferring} onclick={transfer}>{m['claims.transfer_code']()}</Button>
		{/snippet}
	</Sheet>

	<Section title={m['codes.section_certificate']()}>
		<CertificatePanel code={c.code} type={c.type} />
	</Section>

	<Section title={m['codes.section_raw']()}>
		<RecordInspector record={detail} labels={inspectorLabels()} />
	</Section>
{/if}

<style>
	.transfer {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		max-width: 40rem;
	}
	.transfer textarea,
	.transfer input {
		width: 100%;
		box-sizing: border-box;
	}
	.stats {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05) var(--space-15);
		margin: 0 0 var(--space-1);
		padding: 0;
		list-style: none;
	}
	/* big numbers: the title size, the only size above 1rem (DESIGN.md "Brutalist tools") */
	.stats li {
		display: flex;
		align-items: baseline;
		gap: var(--space-05);
	}
	.stats strong {
		font-size: var(--type-title-size);
		line-height: var(--type-title-line);
	}
</style>
