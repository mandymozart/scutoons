<script lang="ts">
	import { infoBar } from '$lib/info.ts';
	import PageHelp from '$lib/components/page-help.svelte';
	import { onDestroy } from 'svelte';
	import { page } from '$app/state';
	import { ActionBar, Badge, Button, Field, RecordInspector, Section, Sheet, Skeleton } from '@scutoons/ui';
	import type { AdminClaimDetail, AdminClaimFile } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { choiceLabel, claimStatusLabel, claimTone, itemChoices, openClaims, type ItemChoice } from '$lib/claims.svelte.ts';
	import { errorText, minute } from '$lib/format.ts';
	import { inspectorLabels } from '$lib/labels.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PageHeader from '$lib/components/page-header.svelte';
	import CodeStatus from '$lib/components/code-status.svelte';
	import AccountLink from '$lib/components/account-link.svelte';
	import OwnershipHistory from '$lib/components/ownership-history.svelte';

	/** One ownership claim (GET /admin/claims/{id}): the evidence, the code's history, and the decision (transfer / reject). */
	const toaster = getToaster();
	const id = $derived(page.params.id ?? '');

	let detail = $state<AdminClaimDetail | null>(null);
	let error = $state<string | null>(null);

	$effect(() => {
		const current = id;
		detail = null;
		error = null;
		transferNote = '';
		rejectNote = '';
		choices = null;
		choice = '';
		admin
			.getClaim(current)
			.then((d) => {
				if (current === id) show(d);
			})
			.catch((e) => (error = errorText(e)));
	});

	function show(d: AdminClaimDetail) {
		detail = d;
		void loadFiles(d.claim.id, d.claim.files);
	}

	// ── evidence files: fetched with the session (fetchRaw) → object URLs, revoked on leave ──
	let fileUrls = $state<Record<string, string>>({});
	let fileErrors = $state<Record<string, string>>({});
	const created: string[] = [];

	async function loadFiles(claimId: string, files: AdminClaimFile[]) {
		for (const file of files) {
			if (fileUrls[file.id] || fileErrors[file.id]) continue;
			try {
				const blob = await (await admin.claimFile(claimId, file.id)).blob();
				const url = URL.createObjectURL(blob.type ? blob : new Blob([blob], { type: file.type }));
				created.push(url);
				fileUrls[file.id] = url;
			} catch (e) {
				fileErrors[file.id] = errorText(e);
			}
		}
	}

	onDestroy(() => created.forEach((url) => URL.revokeObjectURL(url)));

	const kb = (size: number) => (size >= 1048576 ? `${(size / 1048576).toFixed(1)} MB` : `${Math.max(1, Math.round(size / 1024))} KB`);
	const isImage = (file: AdminClaimFile) => file.type.startsWith('image/');

	// ── decision ──
	let transferNote = $state('');
	let rejectNote = $state('');
	let choices = $state<ItemChoice[] | null>(null);
	let choice = $state('');
	let confirming = $state<'transfer' | 'reject' | null>(null);
	let confirmOpen = $state(false);
	let working = $state(false);

	const isOpen = $derived(detail?.claim.status === 'open');
	const canTransfer = $derived(!!transferNote.trim() && (!choices || choice !== ''));

	function ask(event: SubmitEvent, action: 'transfer' | 'reject') {
		event.preventDefault();
		if (action === 'transfer' ? !canTransfer : !rejectNote.trim()) return;
		confirming = action;
		confirmOpen = true;
	}

	async function decide() {
		if (!detail || !confirming) return;
		const action = confirming;
		working = true;
		try {
			let next: AdminClaimDetail;
			if (action === 'transfer') {
				const picked = choices && choice !== '' ? choices[Number(choice)] : null;
				next = await admin.transferClaim(detail.claim.id, transferNote.trim(), picked ? { item_id: picked.item_id, ...(picked.variant ? { variant: picked.variant } : {}) } : {});
			} else {
				next = await admin.rejectClaim(detail.claim.id, rejectNote.trim());
			}
			confirmOpen = false;
			transferNote = '';
			rejectNote = '';
			choices = null;
			choice = '';
			show(next);
			void openClaims.refresh();
			toaster.addToast({
				message: action === 'transfer' ? m['claims.transferred']({ code: next.claim.code, email: next.claim.claimant?.email ?? '—' }) : m['claims.rejected']({ code: next.claim.code }),
				type: 'success'
			});
		} catch (e) {
			confirmOpen = false;
			const options = itemChoices(e);
			if (options) {
				choices = options;
				choice = '';
				toaster.addToast({ message: m['claims.choose_item_needed'](), type: 'error' });
			} else {
				toaster.addToast({ message: errorText(e), type: 'error' });
			}
		} finally {
			working = false;
		}
	}

	const cert = $derived(detail?.certificate.certificate ?? null);
	const ownerChanged = $derived(!!detail && (detail.claim.owner?.id ?? null) !== (detail.current_owner?.id ?? null));
</script>

{#if error}
	<PageHeader type={m['claims.type_label']()} title={id} />
	<p class="error-box" role="alert">{error}</p>
{:else if !detail}
	<PageHeader type={m['claims.type_label']()} title={id} />
	<Skeleton lines={10} label={m['shell.loading']()} />
{:else}
	{@const claim = detail.claim}
	{@const code = detail.code}
	<PageHeader
		type={m['claims.type_label']()}
		title={claim.code}
		meta={claim.decided_at ? m['claims.meta_decided']({ filed: minute(claim.created_at), decided: minute(claim.decided_at) }) : m['claims.meta_filed']({ filed: minute(claim.created_at) })}
	>
		<p class="status"><Badge tone={claimTone(claim.status)}>{claimStatusLabel(claim.status)}</Badge></p>
	</PageHeader>

	<ActionBar label={m['shell.page_actions']()} moreLabel={m['shell.more']()} {...infoBar()} infoTitle={m['claims.type_label']()}>
		{#snippet info()}<PageHelp page="claim" />{/snippet}
		{#snippet start()}
			<a href="/claims">{m['claims.back']()}</a>
		{/snippet}
		{#snippet secondary()}
			<Button size="sm" variant="secondary" href="/claims?code={encodeURIComponent(claim.code)}">{m['claims.for_this_code']()}</Button>
			{#if isOpen}
				<Button size="sm" variant="secondary" type="submit" form="reject-form" disabled={!rejectNote.trim() || working}>{m['claims.reject']()}</Button>
			{/if}
		{/snippet}
		{#snippet end()}
			{#if isOpen}
				<Button size="sm" type="submit" form="transfer-form" disabled={!canTransfer || working}>{m['claims.transfer']()}</Button>
			{/if}
		{/snippet}
	</ActionBar>

	<div class="grid-2">
		<Section title={m['claims.section_claim']()}>
			<dl class="facts">
				<dt>{m['claims.col_filed']()}</dt>
				<dd>{minute(claim.created_at)}</dd>
				<dt>{m['claims.col_claimant']()}</dt>
				<dd><AccountLink account={claim.claimant} /></dd>
				<dt>{m['claims.col_owner']()}</dt>
				<dd>
					{#if claim.owner}<AccountLink account={claim.owner} />
					{:else if detail.legacy_registration}<span class="muted">{m['claims.legacy_registration']()}</span>
					{:else}—{/if}
				</dd>
				<dt>{m['claims.current_owner']()}</dt>
				<dd>
					{#if detail.current_owner}<AccountLink account={detail.current_owner} />
					{:else if detail.legacy_registration}<span class="muted">{m['claims.legacy_registration']()}</span>
					{:else}—{/if}
					{#if ownerChanged}<Badge tone="warning">{m['claims.owner_changed']()}</Badge>{/if}
				</dd>
				{#if claim.decided_at}
					<dt>{m['claims.col_decided']()}</dt>
					<dd>{minute(claim.decided_at)}{#if claim.admin}{' · '}<AccountLink account={claim.admin} />{/if}</dd>
				{/if}
				{#if claim.admin_note}
					<dt>{m['claims.admin_note']()}</dt>
					<dd class="pre">{claim.admin_note}</dd>
				{/if}
			</dl>
		</Section>

		<Section title={m['claims.section_code']()}>
			{#if code}
				<dl class="facts">
					<dt>{m['codes.col_code']()}</dt>
					<dd><a class="mono-code" href="/codes/{encodeURIComponent(code.code.code)}">{code.code.code}</a></dd>
					<dt>{m['common.edition']()}</dt>
					<dd>{code.edition ? `${code.edition.name} #${code.code.edition_number ?? '—'}` : m['common.no_edition']()}</dd>
					<dt>{m['codes.col_type']()}</dt>
					<dd>{code.code.type}</dd>
					<dt>{m['codes.col_status']()}</dt>
					<dd><CodeStatus claimed={code.code.claimed} revoked={code.revoked} expired={code.expired} signed={code.signed} /></dd>
					<dt>{m['codes.col_claimed']()}</dt>
					<dd>{code.code.claimed ? minute(code.code.claimed_at) : '—'}</dd>
					<dt>{m['codes.items']()}</dt>
					<dd>
						{#each code.items as item, i (i)}
							<Badge>{item.item_id}{item.variant ? ` · ${item.variant}` : ''} × {item.qty}</Badge>
						{:else}—{/each}
					</dd>
				</dl>
			{:else}
				<p class="muted">{m['claims.code_gone']()}</p>
			{/if}
		</Section>
	</div>

	<Section title={m['claims.section_statement']()} hint={m['claims.section_statement_hint']()}>
		<p class="pre statement" data-testid="claim-statement">{claim.statement}</p>
	</Section>

	<Section title={m['claims.section_files']()} hint={m['claims.section_files_hint']()}>
		{#if claim.files.length}
			<ul class="files" data-testid="claim-files">
				{#each claim.files as file (file.id)}
					<li>
						{#if fileErrors[file.id]}
							<span class="error-text">{file.name}: {fileErrors[file.id]}</span>
						{:else if !fileUrls[file.id]}
							<span class="muted">{file.name} …</span>
						{:else if isImage(file)}
							<a class="thumb" href={fileUrls[file.id]} target="_blank" rel="noreferrer" title={m['claims.open_file']({ name: file.name })}>
								<img src={fileUrls[file.id]} alt={file.name} />
							</a>
						{:else}
							<a class="doc" href={fileUrls[file.id]} target="_blank" rel="noreferrer">{file.name}</a>
						{/if}
						<span class="file-meta">{file.name} · {file.type.replace('application/', '').replace('image/', '').toUpperCase()} · {kb(file.size)}</span>
					</li>
				{/each}
			</ul>
		{:else}
			<p class="muted">{m['claims.no_files']()}</p>
		{/if}
	</Section>

	<Section title={m['claims.section_certificate']()} hint={m['claims.section_certificate_hint']()}>
		<dl class="facts" data-testid="claim-certificate">
			<dt>{m['claims.cert_result']()}</dt>
			<dd>
				<Badge tone={detail.certificate.valid ? 'published' : 'error'}>{detail.certificate.valid ? m['claims.cert_valid']() : m['claims.cert_invalid']()}</Badge>
				{#if detail.certificate.reason}<span class="muted">{detail.certificate.reason}</span>{/if}
			</dd>
			{#if detail.certificate.registered !== undefined && detail.certificate.registered !== null}
				<dt>{m['claims.cert_registered']()}</dt>
				<dd>{detail.certificate.registered ? m['common.yes']() : m['common.no']()}</dd>
			{/if}
			{#if detail.certificate.revoked !== undefined && detail.certificate.revoked !== null}
				<dt>{m['codes.status_revoked']()}</dt>
				<dd>{detail.certificate.revoked ? m['common.yes']() : m['common.no']()}</dd>
			{/if}
			{#if cert?.product_name}
				<dt>{m['claims.cert_product']()}</dt>
				<dd>{cert.product_name}{cert.edition_number ? ` · #${cert.edition_number}${cert.edition_count ? ` / ${cert.edition_count}` : ''}` : ''}</dd>
			{/if}
			{#if cert?.fingerprint}
				<dt>{m['claims.cert_fingerprint']()}</dt>
				<dd class="mono">{cert.fingerprint}</dd>
			{/if}
			{#if detail.certificate.error}
				<dt>{m['claims.cert_error']()}</dt>
				<dd class="error-text">{detail.certificate.error}</dd>
			{/if}
		</dl>
	</Section>

	{#if isOpen}
		<Section title={m['claims.section_decision']()} hint={m['claims.section_decision_hint']()}>
			<div class="grid-2">
				<form id="transfer-form" class="decision" onsubmit={(e) => ask(e, 'transfer')}>
					<h3 class="sc-label">{m['claims.transfer']()}</h3>
					<p class="muted">{m['claims.transfer_explain']({ email: claim.claimant?.email ?? '—' })}</p>
					<Field label={m['claims.note']()} hint={m['claims.transfer_note_hint']()} required>
						{#snippet children({ id, describedby })}
							<textarea {id} aria-describedby={describedby} rows="3" maxlength="2000" bind:value={transferNote} data-testid="transfer-note"></textarea>
						{/snippet}
					</Field>
					{#if choices}
						<Field label={m['claims.choose_item']()} hint={m['claims.choose_item_hint']()} required>
							{#snippet children({ id, describedby })}
								<select {id} aria-describedby={describedby} bind:value={choice}>
									<option value="">{m['claims.choose_item_placeholder']()}</option>
									{#each choices ?? [] as c, i (i)}
										<option value={String(i)}>{choiceLabel(c)}</option>
									{/each}
								</select>
							{/snippet}
						</Field>
					{/if}
				</form>
				<form id="reject-form" class="decision" onsubmit={(e) => ask(e, 'reject')}>
					<h3 class="sc-label">{m['claims.reject']()}</h3>
					<p class="muted">{m['claims.reject_explain']()}</p>
					<Field label={m['claims.note']()} hint={m['claims.reject_note_hint']()} required>
						{#snippet children({ id, describedby })}
							<textarea {id} aria-describedby={describedby} rows="3" maxlength="2000" bind:value={rejectNote} data-testid="reject-note"></textarea>
						{/snippet}
					</Field>
				</form>
			</div>
		</Section>
	{/if}

	<Section title={m['claims.section_history']()} hint={m['claims.section_history_hint']()}>
		<OwnershipHistory items={detail.history} />
	</Section>

	<Section title={m['claims.section_other']()}>
		{#if detail.other_claims.length}
			<ul class="others">
				{#each detail.other_claims as other (other.id)}
					<li>
						<Badge tone={claimTone(other.status)}>{claimStatusLabel(other.status)}</Badge>
						<a href="/claims/{encodeURIComponent(other.id)}">{minute(other.created_at)}</a>
						<span class="muted">{m['claims.by']()}</span>
						<AccountLink account={other.claimant} />
					</li>
				{/each}
			</ul>
		{:else}
			<p class="muted">{m['claims.no_other']()}</p>
		{/if}
	</Section>

	<Section title={m['codes.section_raw']()}>
		<RecordInspector record={detail} labels={inspectorLabels()} />
	</Section>

	<Sheet bind:open={confirmOpen} title={confirming === 'transfer' ? m['claims.confirm_transfer_title']() : m['claims.confirm_reject_title']()} closeLabel={m['common.close']()}>
		{#if confirming === 'transfer'}
			<p>{m['claims.confirm_transfer']({ code: claim.code, email: claim.claimant?.email ?? '—' })}</p>
			{#if choices && choice !== ''}<p>{m['claims.choose_item']()}: {choiceLabel(choices[Number(choice)])}</p>{/if}
		{:else}
			<p>{m['claims.confirm_reject']({ code: claim.code, email: claim.claimant?.email ?? '—' })}</p>
		{/if}
		{#snippet footer()}
			<Button size="sm" variant="secondary" onclick={() => (confirmOpen = false)}>{m['common.cancel']()}</Button>
			<Button size="sm" disabled={working} onclick={decide} data-testid="confirm-decision">
				{confirming === 'transfer' ? m['claims.transfer']() : m['claims.reject']()}
			</Button>
		{/snippet}
	</Sheet>
{/if}

<style>
	.status {
		margin: var(--space-05) 0 0;
	}
	.pre {
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}
	.statement {
		margin: 0;
		max-width: 48rem;
	}
	.mono {
		overflow-wrap: anywhere;
	}
	.error-text {
		overflow-wrap: anywhere;
	}
	.files {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-1);
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.files li {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		max-width: 14rem;
	}
	.thumb {
		display: block;
		border: var(--border-width-regular) solid var(--ink);
	}
	.thumb img {
		display: block;
		width: 14rem;
		height: 10rem;
		object-fit: cover;
	}
	.doc {
		display: flex;
		align-items: center;
		justify-content: center;
		box-sizing: border-box;
		width: 14rem;
		height: 10rem;
		padding: var(--space-05);
		border: var(--border-width-regular) solid var(--ink);
		overflow-wrap: anywhere;
		text-align: center;
	}
	.file-meta {
		color: var(--text-muted);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		overflow-wrap: anywhere;
	}
	.decision {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
	}
	.decision h3,
	.decision p {
		margin: 0;
	}
	.decision textarea {
		width: 100%;
		box-sizing: border-box;
	}
	.others {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.others li {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-05);
	}
</style>
