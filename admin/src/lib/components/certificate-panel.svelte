<script lang="ts">
	import { Badge, Button, Skeleton } from '@scutoons/ui';
	import { ApiError, type CodeSignatureResult } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { errorText } from '$lib/format.ts';
	import { qrUrl } from '$lib/qr/config.ts';
	import { verifySignature } from '$lib/qr/crypto.ts';
	import { saveBlob, saveText } from '$lib/qr/download.ts';
	import { coaPdf, qrPrintPdf, type CertificateData } from '$lib/qr/pdf.ts';
	import { qrSvg } from '$lib/qr/qr.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';
	import QrPreview from './qr-preview.svelte';

	/**
	 * "Regenerate" of the legacy SPA: the certificate files of an existing code, rebuilt from its stored signature
	 * (`qr_data` = the signed data, so the PDF shows exactly what was signed). Same download names as legacy regenerate.
	 */
	let { code, type }: { code: string; type: string } = $props();

	const toaster = getToaster();
	let result = $state<CodeSignatureResult | null>(null);
	let missing = $state(false);
	let error = $state<string | null>(null);
	let loading = $state(true);

	$effect(() => {
		const current = code;
		loading = true;
		result = null;
		missing = false;
		error = null;
		admin
			.getSignature(current)
			.then((r) => (result = r))
			.catch((e) => {
				if (e instanceof ApiError && e.code === 'no_signature') missing = true;
				else error = errorText(e);
			})
			.finally(() => (loading = false));
	});

	/** qr_data + signature + fingerprint + redirect_to, like legacy regenerate */
	const qrData = $derived<CertificateData | null>(
		result?.qr_data
			? ({
					...result.qr_data,
					signature: result.signature,
					fingerprint: result.fingerprint,
					redirect_to: result.redirect_to
				} as unknown as CertificateData)
			: null
	);
	const url = $derived(qrData?.url ?? qrUrl(code));
	const svg = $derived(qrSvg(url));
	const certType = $derived(qrData?.type ?? type);
	// against the server's key (not the one embedded in the signature) and for this very code
	let verified = $state(false);
	$effect(() => {
		const current = result;
		verified = false;
		if (!current) return;
		verifySignature(current.signature, code).then((ok) => {
			if (result === current) verified = ok;
		});
	});

	function downloadSvg() {
		saveText(svg, `qr-${code}.svg`, 'image/svg+xml');
	}

	async function downloadPdf(kind: 'coa' | 'print') {
		if (!qrData) return;
		try {
			const doc = kind === 'coa' ? await coaPdf(qrData, qrData, svg) : await qrPrintPdf(qrData, qrData, svg);
			saveBlob(doc.output('blob'), kind === 'coa' ? `certificate-${code}.pdf` : `qr-print-${code}.pdf`);
		} catch (e) {
			toaster.addToast({ message: errorText(e), type: 'error' });
		}
	}

	function downloadSignature() {
		if (!qrData?.signature) return;
		const json = JSON.stringify({ ...qrData.signature, code: qrData.code, type: qrData.type, created_at: qrData.created_at }, null, 2);
		saveText(json, `signature-${code}.json`, 'application/json');
	}
</script>

{#if loading}
	<Skeleton lines={4} label={m['shell.loading']()} />
{:else if error}
	<p class="error-box" role="alert">{error}</p>
{:else}
	<div class="certificate">
		<QrPreview {svg} {url} label={m['codes.qr_of']({ code })} />
		<div class="info">
			{#if result && qrData}
				<dl class="facts">
					<dt>{m['codes.algorithm']()}</dt>
					<dd>{result.signature.algorithm}</dd>
					<dt>{m['codes.fingerprint']()}</dt>
					<dd class="fingerprint">{result.fingerprint}</dd>
					<dt>{m['codes.signed_at']()}</dt>
					<dd>{result.signature.timestamp}</dd>
					<dt>{m['codes.verification']()}</dt>
					<dd>
						{#if verified}<Badge tone="accent">{m['codes.signature_valid']()}</Badge>{:else}<Badge tone="error">{m['codes.signature_invalid']()}</Badge>{/if}
					</dd>
				</dl>
			{:else if missing}
				<p class="muted">{m['codes.no_signature']()}</p>
			{/if}
			<div class="downloads">
				<Button size="sm" variant="secondary" onclick={downloadSvg}>{m['codes.download_svg']()}</Button>
				{#if qrData && certType === 'membership'}
					<Button size="sm" variant="secondary" onclick={() => downloadPdf('coa')}>{m['codes.download_coa']()}</Button>
				{/if}
				{#if qrData && certType === 'promo'}
					<Button size="sm" variant="secondary" onclick={() => downloadPdf('print')}>{m['codes.download_print']()}</Button>
				{/if}
				{#if qrData}
					<Button size="sm" variant="secondary" onclick={downloadSignature}>{m['codes.download_signature']()}</Button>
				{/if}
			</div>
		</div>
	</div>
{/if}

<style>
	.certificate {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-15);
		align-items: flex-start;
	}
	.info {
		flex: 1 1 18rem;
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		min-width: 0;
	}
	.fingerprint {
		font-size: var(--type-caption-size);
	}
	.downloads {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05);
	}
</style>
