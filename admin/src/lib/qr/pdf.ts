// Certificate PDFs, port of generate/app/utils/pdf.js: jsPDF (same version), courier 9pt typewriter layout, the QR
// image (SVG → 256px canvas → PNG) at the template's {{qr}} line, the post-quantum footer at the bottom.
import { jsPDF } from 'jspdf';
import type { CodeSignature } from '@scutoons/api';
import { QR_DOMAIN } from './config.ts';
import { renderTemplate, TEMPLATES } from './template.ts';

/** QR data as signed: code, type, redirect_to, url, created_at, expires_at, template fields (+ signature after signing) */
export interface CertificateData {
	code: string;
	type: string;
	url: string;
	created_at: string;
	expires_at?: string | null;
	signature?: CodeSignature;
	fingerprint?: string;
	[key: string]: unknown;
}

/** SVG markup → PNG data URL on a 256×256 canvas (as the legacy code did) */
export function svgToPng(svg: string): Promise<string> {
	return new Promise((resolve, reject) => {
		const canvas = document.createElement('canvas');
		canvas.width = 256;
		canvas.height = 256;
		const ctx = canvas.getContext('2d');
		const img = new Image();
		const url = URL.createObjectURL(new Blob([svg], { type: 'image/svg+xml;charset=utf-8' }));
		img.onload = () => {
			ctx?.drawImage(img, 0, 0);
			URL.revokeObjectURL(url);
			resolve(canvas.toDataURL('image/png'));
		};
		img.onerror = () => {
			URL.revokeObjectURL(url);
			reject(new Error('Failed to load QR code image'));
		};
		img.src = url;
	});
}

async function renderTemplateToPdf(doc: jsPDF, rendered: string, svg: string, qrData: CertificateData, qrCodeSize = 40) {
	const margin = 15;
	const pageWidth = doc.internal.pageSize.getWidth();
	const pageHeight = doc.internal.pageSize.getHeight();
	let y = margin;
	let qrCodeY: number | null = null;

	doc.setFontSize(9);
	doc.setFont('courier', 'normal');

	for (const raw of rendered.split('\n')) {
		const line = raw.trim();
		if (line === '') {
			y += 4;
			continue;
		}
		if (line === '[QR_CODE_PLACEHOLDER]') {
			qrCodeY = y;
			y += qrCodeSize + 3;
			continue;
		}
		if (line.startsWith('___')) {
			const lineWidth = (pageWidth - 2 * margin) / 3;
			doc.line(margin, y, margin + lineWidth, y);
			y += 5;
			continue;
		}
		const split = doc.splitTextToSize(line, pageWidth - 2 * margin);
		doc.text(split, margin, y);
		y += split.length * 4.5 + 1;
	}

	if (qrCodeY === null) throw new Error('The template has no {{qr}} line');
	doc.addImage(await svgToPng(svg), 'PNG', margin, qrCodeY, qrCodeSize, qrCodeSize);

	if (qrData.signature) {
		let yBottom = pageHeight - 15;
		doc.setFontSize(7);
		doc.setFont('courier', 'normal');
		yBottom -= 3;
		doc.text('This certificate is protected by post-quantum cryptography (ML-DSA-65/Dilithium).', margin, yBottom);
		yBottom -= 4;
		// PRINTED: verify URL + the code exactly as it is – codes are case-sensitive, verified by exact match only
		// (owner 2026-10-03; certificates printed before printed it in UPPER CASE – agents/PLATFORM.md printed-URL rule)
		doc.text('Verify at ' + QR_DOMAIN + '/verify.html with code: ' + qrData.code, margin, yBottom);
		yBottom -= 4;
		doc.text(`Fingerprint: ${qrData.fingerprint}`, margin, yBottom);
		yBottom -= 4;
		doc.text(
			`Algorithm: ${qrData.signature.algorithm} | Timestamp: ${new Date(qrData.signature.timestamp).toLocaleString()}`,
			margin,
			yBottom
		);
		yBottom -= 5;
		doc.text('Post-Quantum Cryptographic Verification', margin, yBottom);
	}
	return doc;
}

async function pdfFromTemplate(template: string, qrData: CertificateData, templateData: Record<string, unknown>, svg: string) {
	const formatted = {
		...qrData,
		...templateData,
		created_at: new Date(qrData.created_at).toLocaleDateString(),
		expires_at: qrData.expires_at ? 'Expires: ' + new Date(qrData.expires_at).toLocaleDateString() : ''
	};
	const doc = new jsPDF();
	doc.setFont('courier', 'normal');
	return renderTemplateToPdf(doc, renderTemplate(template, formatted), svg, qrData, 50);
}

/** Certificate of Authenticity (membership, template cos.md) */
export const coaPdf = (qrData: CertificateData, templateData: Record<string, unknown>, svg: string) =>
	pdfFromTemplate(TEMPLATES.cos, qrData, templateData, svg);

/** Printable QR / access certificate (promo, template qr.md) */
export const qrPrintPdf = (qrData: CertificateData, templateData: Record<string, unknown>, svg: string) =>
	pdfFromTemplate(TEMPLATES.qr, qrData, templateData, svg);
