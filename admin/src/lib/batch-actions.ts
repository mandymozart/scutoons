// Batch downloads shared by the batch list and the batch page: ZIP (server-side) and the sticker print sheet.
import { admin } from '$lib/api.ts';
import { saveBlob, saveResponse } from '$lib/qr/download.ts';
import { printSheet, printSheetFilename } from '$lib/qr/print-sheet.ts';

export async function downloadZip(filename: string) {
	await saveResponse(await admin.batchZip(filename), filename.replace('.json', '.zip'));
}

export async function downloadPrintSheet(filename: string) {
	const { batch } = await admin.getBatch(filename);
	const doc = await printSheet(batch);
	saveBlob(doc.output('blob'), printSheetFilename(filename));
}
