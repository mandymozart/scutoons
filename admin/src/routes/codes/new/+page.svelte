<script lang="ts">
	import { ActionBar, Badge, Button, Field, Panel, Section } from '@scutoons/ui';
	import type { CodeSignature } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { CODE_TYPES, loadCatalog, type Edition } from '$lib/catalog.ts';
	import { errorText } from '$lib/format.ts';
	import { QR_DOMAIN, qrUrl } from '$lib/qr/config.ts';
	import { fingerprint, signQrData } from '$lib/qr/crypto.ts';
	import { saveBlob, saveText } from '$lib/qr/download.ts';
	import { coaPdf, qrPrintPdf, type CertificateData } from '$lib/qr/pdf.ts';
	import { coaFilename, qrPrintFilename, qrSvg, signatureFilename, svgFilename } from '$lib/qr/qr.ts';
	import { parseTemplate, templateFor } from '$lib/qr/template.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PageHeader from '$lib/components/page-header.svelte';
	import QrPreview from '$lib/components/qr-preview.svelte';

	/**
	 * Port of the legacy "Generate" mode: one code of any type, signed client-side (ML-DSA-65), QR SVG + signature
	 * archived on the server, certificate PDF (membership) / printable QR (promo) on demand.
	 */
	const toaster = getToaster();

	let toons = $state<{ id: string; name: string; redirectUrl: string }[]>([]);
	let allEditions = $state<Edition[]>([]);

	let toon = $state('');
	let type = $state('membership');
	let redirectUrl = $state('');
	let expiresAt = $state('');
	let editionId = $state('');
	let values = $state<Record<string, string>>({});
	let working = $state(false);
	let formError = $state<string | null>(null);

	let current = $state<{
		data: CertificateData;
		templateData: Record<string, string>;
		svg: string;
		signature: CodeSignature | null;
	} | null>(null);

	$effect(() => {
		loadCatalog()
			.then((catalog) => {
				toons = catalog.toons;
				allEditions = catalog.editions as Edition[];
				if (!toon && toons.length) selectToon(toons[0].id);
			})
			.catch((e) => (formError = errorText(e)));
	});

	const editions = $derived(allEditions.filter((e) => e.toon === toon));
	const template = $derived(templateFor(type));
	const fields = $derived(template ? parseTemplate(template) : []);
	const showEdition = $derived(type === 'membership' && editions.length > 0 && fields.length > 0);

	function selectToon(id: string) {
		toon = id;
		const t = toons.find((x) => x.id === id);
		if (t) redirectUrl = t.redirectUrl;
		const first = allEditions.find((e) => e.toon === id);
		if (type === 'membership' && first) selectEdition(first.id);
		else editionId = '';
	}

	/** Edition → template fields (name → productname, edition_number, edition_count, excluded_number, creators) */
	function selectEdition(id: string) {
		editionId = id;
		const e = allEditions.find((x) => x.id === id);
		if (!e) return;
		const map: Record<string, unknown> = {
			productname: e.name,
			edition_number: e.edition_number ?? 1,
			edition_count: e.edition_count,
			excluded_number: e.excluded_number,
			creators: e.creators
		};
		for (const [key, value] of Object.entries(map)) {
			if (value !== undefined && value !== null && typeof value !== 'object') values[key] = String(value);
		}
	}

	function changeType(next: string) {
		type = next;
		if (next === 'membership' && !editionId && editions.length) selectEdition(editions[0].id);
	}

	async function archive(type: 'svgs' | 'certificates' | 'signatures', filename: string, doc: string) {
		try {
			await admin.putFile(type, filename, doc);
		} catch (e) {
			console.error('File save error', filename, e);
			toaster.addToast({ message: m['generate.archive_failed']({ file: filename }), type: 'warning' });
		}
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (working) return;
		working = true;
		formError = null;
		try {
			const created = (
				await admin.createCode({
					redirect_to: redirectUrl.trim(),
					type,
					email: '',
					expires_at: expiresAt || null,
					edition_id: showEdition && editionId ? editionId : null
				})
			).code;

			// template data = every template field of the form, as strings (like the legacy FormData)
			const templateData: Record<string, string> = {};
			for (const field of fields) templateData[field.id] = String(values[field.id] ?? '');

			const data: CertificateData = {
				code: created.code,
				type: created.type,
				redirect_to: created.redirect_to,
				url: qrUrl(created.code),
				created_at: new Date().toISOString(),
				expires_at: created.expires_at || null,
				...templateData,
				svg: null
			};

			let signature: CodeSignature | null = null;
			try {
				signature = await signQrData(data);
				data.signature = signature;
				data.fingerprint = fingerprint(signature);
				const sigJson = JSON.stringify(
					{ code: data.code, type: data.type, signature, fingerprint: data.fingerprint, created_at: data.created_at },
					null,
					2
				);
				await archive('signatures', signatureFilename(data.code), sigJson);
				await admin.saveSignature(data.code, signature);
			} catch (e) {
				console.error('Failed to sign QR code:', e);
				toaster.addToast({ message: m['generate.sign_failed']({ error: errorText(e) }), type: 'warning' });
			}

			const svg = qrSvg(data.url);
			await archive('svgs', svgFilename(data.type, data.code), svg);
			current = { data, templateData, svg, signature };
			toaster.addToast({ message: m['generate.done']({ code: data.code }), type: 'success' });
		} catch (e) {
			formError = errorText(e);
		} finally {
			working = false;
		}
	}

	async function downloadPdf(kind: 'coa' | 'print') {
		if (!current) return;
		try {
			const doc =
				kind === 'coa'
					? await coaPdf(current.data, current.templateData, current.svg)
					: await qrPrintPdf(current.data, current.templateData, current.svg);
			const filename = kind === 'coa' ? coaFilename(current.data.code) : qrPrintFilename(current.data.code);
			saveBlob(doc.output('blob'), filename);
			await archive('certificates', filename, doc.output('datauristring'));
		} catch (e) {
			toaster.addToast({ message: errorText(e), type: 'error' });
		}
	}

	function downloadSignature() {
		if (!current?.signature) return;
		const d = current.data;
		const json = JSON.stringify({ code: d.code, type: d.type, signature: current.signature, fingerprint: d.fingerprint, created_at: d.created_at }, null, 2);
		saveText(json, signatureFilename(d.code), 'application/json');
	}
</script>

<PageHeader type={m['nav.group_codes']()} title={m['generate.title']()} meta={m['generate.meta']()} />

<ActionBar label={m['shell.page_actions']()} moreLabel={m['shell.more']()}>
	{#snippet start()}
		<a href="/codes">← {m['codes.back_to_list']()}</a>
		{#if current}<span class="muted">{m['generate.result']({ code: current.data.code })}</span>{/if}
	{/snippet}
	{#snippet end()}
		<Button size="sm" type="submit" form="generate-form" disabled={working}>
			{working ? m['generate.generating']() : m['generate.submit']()}
		</Button>
	{/snippet}
</ActionBar>

<div class="grid-2">
	<form onsubmit={submit} class="generate-form" id="generate-form">
		<Field label={m['generate.toon']()}>
			{#snippet children({ id })}
				<select {id} value={toon} onchange={(e) => selectToon(e.currentTarget.value)}>
					<option value="">{m['generate.no_toon']()}</option>
					{#each toons as t (t.id)}
						<option value={t.id}>{t.name}</option>
					{/each}
				</select>
			{/snippet}
		</Field>
		<Field label={m['codes.col_type']()}>
			{#snippet children({ id })}
				<select {id} value={type} onchange={(e) => changeType(e.currentTarget.value)}>
					{#each CODE_TYPES as t (t)}
						<option value={t}>{t}</option>
					{/each}
				</select>
			{/snippet}
		</Field>
		<Field label={m['generate.redirect']()} required hint={m['generate.redirect_hint']()}>
			{#snippet children({ id, describedby })}
				<input {id} type="url" required placeholder="https://scutoons.com" aria-describedby={describedby} bind:value={redirectUrl} />
			{/snippet}
		</Field>
		<Field label={m['generate.expires']()} hint={m['generate.expires_hint']()}>
			{#snippet children({ id, describedby })}
				<input {id} type="datetime-local" aria-describedby={describedby} bind:value={expiresAt} />
			{/snippet}
		</Field>

		{#if fields.length}
			<h2>{m['generate.template_fields']()}</h2>
			<p class="muted">{type === 'membership' ? m['generate.template_coa']() : m['generate.template_print']()}</p>
			{#if showEdition}
				<Field label={m['common.edition']()}>
					{#snippet children({ id })}
						<select {id} value={editionId} onchange={(e) => selectEdition(e.currentTarget.value)}>
							<option value="">{m['generate.select_edition']()}</option>
							{#each editions as e (e.id)}
								<option value={e.id}>{e.name}</option>
							{/each}
						</select>
					{/snippet}
				</Field>
			{/if}
			{#each fields as field (field.id)}
				<Field label={field.label}>
					{#snippet children({ id })}
						{#if field.type === 'textarea'}
							<textarea {id} rows="3" placeholder={field.placeholder} bind:value={values[field.id]}></textarea>
						{:else}
							<input {id} type={field.type} placeholder={field.placeholder} bind:value={values[field.id]} />
						{/if}
					{/snippet}
				</Field>
			{/each}
		{/if}

		{#if formError}<p class="error-box" role="alert">{formError}</p>{/if}
	</form>

	<div>
		{#if current}
			{@const d = current.data}
			<Panel title={m['generate.result']({ code: d.code })}>
				<div class="result">
					<QrPreview svg={current.svg} url={d.url} label={m['codes.qr_of']({ code: d.code })} />
					<dl class="facts">
						<dt>{m['codes.qr_url']()}</dt>
						<dd>{d.url}</dd>
						<dt>{m['codes.redirect']()}</dt>
						<dd>{String(d.redirect_to)}</dd>
						<dt>{m['codes.col_code']()}</dt>
						<dd><a class="mono-code" href="/codes/{encodeURIComponent(d.code)}">{d.code}</a></dd>
						{#if current.signature}
							<dt>{m['codes.algorithm']()}</dt>
							<dd>{current.signature.algorithm} <Badge tone="accent">{m['codes.status_signed']()}</Badge></dd>
							<dt>{m['codes.fingerprint']()}</dt>
							<dd class="small">{d.fingerprint}</dd>
							<dt>{m['codes.verification']()}</dt>
							<dd><a href="{QR_DOMAIN}/verify.html" target="_blank" rel="noreferrer">{m['generate.verify_link']()}</a></dd>
						{/if}
					</dl>
					<div class="downloads">
						<Button size="sm" variant="secondary" onclick={() => saveText(current!.svg, svgFilename(d.type, d.code), 'image/svg+xml')}>
							{m['codes.download_svg']()}
						</Button>
						{#if d.type === 'membership'}
							<Button size="sm" variant="secondary" onclick={() => downloadPdf('coa')}>{m['codes.download_coa']()}</Button>
						{/if}
						{#if d.type === 'promo'}
							<Button size="sm" variant="secondary" onclick={() => downloadPdf('print')}>{m['codes.download_print']()}</Button>
						{/if}
						{#if current.signature}
							<Button size="sm" variant="secondary" onclick={downloadSignature}>{m['codes.download_signature']()}</Button>
						{/if}
					</div>
				</div>
			</Panel>
		{:else}
			<Section title={m['generate.how_title']()} hint={m['generate.how_hint']()}>
				<ol class="steps">
					<li>{m['generate.how_1']()}</li>
					<li>{m['generate.how_2']()}</li>
					<li>{m['generate.how_3']()}</li>
				</ol>
			</Section>
		{/if}
	</div>
</div>

<style>
	.generate-form h2 {
		margin: var(--space-15) 0 var(--space-025);
	}
	.generate-form > p {
		margin: 0 0 var(--space-1);
	}
	.result {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}
	.small {
		font-size: var(--type-caption-size);
	}
	.downloads {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05);
	}
	.steps {
		margin: 0;
		padding-left: 1.2em;
	}
</style>
