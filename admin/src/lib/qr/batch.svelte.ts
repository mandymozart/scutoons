// Resumable batch pipeline, port of generate/app/views/batch.js on the admin API. Per code:
// sign → register (POST /admin/codes) → QR SVG (+ store) → certificate PDF (+ store) → signature JSON (+ PUT
// signature) → PUT the manifest. Completed steps live in the manifest (`completedSteps`), so a reload resumes.
// A promo batch (manifest `type: 'promo'`, owner 2026-10-14 "promo batches like editions") has no certificate: its
// pipeline skips the PDF step; the codes are type promo, carry the batch's expiry and qr.batch = the manifest file.
import { ApiError, type BatchManifest, type BatchManifestCode, type BatchStep } from '@scutoons/api';
import { admin } from '$lib/api.ts';
import { qrUrl } from './config.ts';
import { fingerprint, signQrData } from './crypto.ts';
import { coaPdf, type CertificateData } from './pdf.ts';
import { coaFilename, qrSvg, signatureFilename, svgFilename } from './qr.ts';

export type PipelineStep = 'sign' | BatchStep;
export const PIPELINE: PipelineStep[] = ['sign', 'db', 'svg', 'pdf', 'sig'];

/** membership (older manifests carry no type) | promo */
export const batchType = (manifest: BatchManifest) => manifest.type ?? 'membership';
export const isPromo = (manifest: BatchManifest) => batchType(manifest) === 'promo';

export type RunState = 'idle' | 'running' | 'paused' | 'stopped' | 'complete';

export class BatchRun {
	manifest: BatchManifest;
	state = $state<RunState>('idle');
	/** code number → current step while running */
	current = $state<number | null>(null);
	/** code number → steps done in this session (sign is never stored) */
	signed = $state<Record<number, boolean>>({});
	errors = $state<Record<number, string>>({});
	private abortRequested = false;

	constructor(manifest: BatchManifest) {
		this.manifest = $state(manifest);
	}

	get done() {
		return this.manifest.codes.filter((c) => c.status === 'done').length;
	}
	get failed() {
		return this.manifest.codes.filter((c) => c.status === 'error').length;
	}
	get complete() {
		return this.done === this.manifest.total;
	}
	/** the steps this batch runs (a promo batch makes no certificate PDF) */
	get pipeline(): PipelineStep[] {
		return isPromo(this.manifest) ? PIPELINE.filter((s) => s !== 'pdf') : PIPELINE;
	}

	stepDone(item: BatchManifestCode, step: PipelineStep): boolean {
		if (item.status === 'done') return true;
		if (step === 'sign') return !!this.signed[item.n];
		return item.completedSteps.includes(step);
	}

	pause() {
		if (this.state === 'running') this.state = 'paused';
		else if (this.state === 'paused') this.state = 'running';
	}

	abort() {
		this.abortRequested = true;
	}

	async run() {
		if (this.state === 'running') return;
		this.state = 'running';
		this.abortRequested = false;
		for (const item of this.manifest.codes) {
			if (item.status === 'done') continue;
			while ((this.state as RunState) === 'paused' && !this.abortRequested) await new Promise((r) => setTimeout(r, 300));
			if (this.abortRequested) break;

			this.current = item.n;
			item.status = 'running';
			try {
				await this.processCode(item);
				item.status = 'done';
				delete this.errors[item.n];
			} catch (error) {
				console.error(`[batch] code ${item.code} failed:`, error);
				item.status = 'error';
				this.errors[item.n] = error instanceof Error ? error.message : String(error);
			}
			await this.save();
		}
		this.current = null;
		this.state = this.complete ? 'complete' : 'stopped';
	}

	private async save() {
		await admin.saveBatch($state.snapshot(this.manifest) as BatchManifest);
	}

	private mark(item: BatchManifestCode, step: BatchStep) {
		if (!item.completedSteps.includes(step)) item.completedSteps.push(step);
	}

	private async processCode(item: BatchManifestCode) {
		const m = this.manifest;
		const type = batchType(m);
		const promo = type === 'promo';
		const url = qrUrl(item.code);
		const qrData: CertificateData = promo
			? {
					code: item.code,
					type,
					redirect_to: m.redirectUrl,
					url,
					created_at: new Date().toISOString(),
					expires_at: m.expires_at ?? null,
					productname: m.productname
				}
			: {
					code: item.code,
					type,
					redirect_to: m.redirectUrl,
					url,
					created_at: new Date().toISOString(),
					expires_at: null,
					productname: m.productname,
					edition_number: item.n,
					edition_count: m.edition_count,
					excluded_number: m.excluded_number,
					creators: m.creators,
					receiver: ''
				};

		// 1 · sign (every run; the stored certificate and signature must come from the same signature, so a code whose
		// signature step is still open also redoes its certificate)
		const signature = await signQrData(qrData);
		qrData.signature = signature;
		qrData.fingerprint = fingerprint(signature);
		this.signed[item.n] = true;
		if (!item.completedSteps.includes('sig')) item.completedSteps = item.completedSteps.filter((s) => s !== 'pdf');

		// 2 · register in the DB (an existing code counts as registered, like the legacy "already exists")
		if (!item.completedSteps.includes('db')) {
			try {
				await admin.createCode({
					code: item.code,
					redirect_to: m.redirectUrl,
					type,
					email: '',
					edition_id: promo ? null : m.editionId,
					...(promo ? { expires_at: m.expires_at ?? null } : {}),
					batch: m.filename
				});
			} catch (error) {
				if (!(error instanceof ApiError && error.code === 'code_exists')) throw new Error('DB: ' + (error as Error).message);
			}
			this.mark(item, 'db');
		}

		// 3 · QR SVG
		const svg = qrSvg(url);
		if (!item.completedSteps.includes('svg')) {
			await admin.putFile('svgs', svgFilename(type, item.code), svg);
			this.mark(item, 'svg');
		}

		// 4 · certificate PDF (none for a promo batch)
		if (!promo && !item.completedSteps.includes('pdf')) {
			const templateData = {
				productname: m.productname,
				edition_number: item.n,
				edition_count: m.edition_count,
				excluded_number: m.excluded_number,
				creators: m.creators,
				receiver: ''
			};
			const doc = await coaPdf(qrData, templateData, svg);
			await admin.putFile('certificates', coaFilename(item.code), doc.output('datauristring'));
			this.mark(item, 'pdf');
		}

		// 5 · signature JSON + DB
		if (!item.completedSteps.includes('sig')) {
			const json = JSON.stringify(
				{ code: item.code, type, signature, fingerprint: qrData.fingerprint, created_at: qrData.created_at },
				null,
				2
			);
			await admin.putFile('signatures', signatureFilename(item.code), json);
			await admin.saveSignature(item.code, signature);
			this.mark(item, 'sig');
		}
	}
}
