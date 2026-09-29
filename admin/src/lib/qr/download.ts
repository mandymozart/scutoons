/** Saves a blob as a file download in the browser */
export function saveBlob(blob: Blob, filename: string) {
	const url = URL.createObjectURL(blob);
	const a = document.createElement('a');
	a.href = url;
	a.download = filename;
	document.body.appendChild(a);
	a.click();
	a.remove();
	setTimeout(() => URL.revokeObjectURL(url), 1000);
}

export const saveText = (text: string, filename: string, type: string) => saveBlob(new Blob([text], { type }), filename);

/** Downloads a raw API response (CSV, ZIP) under the server's attachment name or a fallback */
export async function saveResponse(response: Response, fallback: string) {
	const disposition = response.headers.get('Content-Disposition') ?? '';
	const match = /filename\*?=(?:UTF-8'')?"?([^";]+)"?/i.exec(disposition);
	saveBlob(await response.blob(), match ? decodeURIComponent(match[1]) : fallback);
}
