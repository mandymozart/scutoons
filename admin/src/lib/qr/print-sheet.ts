// Sticker print sheet of a batch, port of generate/app/utils/printSheet.js: A4, 4×6 stickers of 40 mm (gap 6 mm),
// QR 26 mm centered, "n / total" above and the code below in courier 10pt.
import { jsPDF } from 'jspdf';
import type { BatchManifest } from '@scutoons/api';
import { qrUrl } from './config.ts';
import { qrSvg } from './qr.ts';
import { svgToPng } from './pdf.ts';

const COLS = 4;
const ROWS = 6;
const PER_PAGE = COLS * ROWS;
const STICKER_MM = 40;
const GAP = 6;
const PAGE_W = 210;
const PAGE_H = 297;
const MARGIN_X = (PAGE_W - (COLS * STICKER_MM + (COLS - 1) * GAP)) / 2;
const MARGIN_Y = (PAGE_H - (ROWS * STICKER_MM + (ROWS - 1) * GAP)) / 2;
const QR_MM = 26;
const TEXT_GAP = 1.5;

export async function printSheet(manifest: BatchManifest) {
	const doc = new jsPDF({ unit: 'mm', format: [PAGE_W, PAGE_H] });
	const codes = manifest.codes.filter((c) => c.status === 'done');

	for (let i = 0; i < codes.length; i++) {
		const item = codes[i];
		const pos = i % PER_PAGE;
		if (i > 0 && pos === 0) doc.addPage();
		const col = pos % COLS;
		const row = Math.floor(pos / COLS);
		const cx = MARGIN_X + col * (STICKER_MM + GAP) + STICKER_MM / 2;
		const cy = MARGIN_Y + row * (STICKER_MM + GAP) + STICKER_MM / 2;
		const qrX = cx - QR_MM / 2;
		const qrY = cy - QR_MM / 2;
		doc.addImage(await svgToPng(qrSvg(qrUrl(item.code))), 'PNG', qrX, qrY, QR_MM, QR_MM);

		doc.setFont('courier', 'normal');
		doc.setTextColor(60);
		doc.setFontSize(10);
		doc.text(`${item.n} / ${manifest.total}`, cx, qrY - TEXT_GAP + 3, { align: 'center' });
		doc.setFontSize(10);
		doc.text(item.code, cx, qrY + QR_MM + TEXT_GAP, { align: 'center' });
	}
	doc.setTextColor(0);
	return doc;
}

/** `BATCH-….json` → `BATCH-…-PRINT.pdf` */
export const printSheetFilename = (filename: string) => filename.replace('.json', '-PRINT.pdf');
