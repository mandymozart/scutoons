// The SCUTOONS media standard (owner 2026-10-11: "wir haben einen Standard definiert, mit dem wir möglichst performante
// Dateien haben – die Leute können andere benutzen, aber wir empfehlen das Umkonvertieren"): ONE place that says whether
// a file meets it and turns it into one that does. Every picker / importer goes through here (the asset picker's
// `optimize`, the map, character, slice and catalog importers), and a later optimisation tool in the admin / creator
// (BACKLOG 10.64) will too.
//   images: PNG / JPEG → WebP (transparency kept); WebP, SVG, GIF stay as they are
//   video:  the standard is MP4 (H.264) / WebM in 480p / 720p / 1080p – converting needs ffmpeg, which the server
//           doesn't have: not converted here yet (the place for it, e.g. ffmpeg.wasm in the browser or a worker)
import { toWebp } from './media-upload.ts';

export type MediaKind = 'image' | 'video' | 'audio' | 'other';

export interface MediaVerdict {
	/** the file meets the standard as it is */
	ok: boolean;
	/** what converting it would do (for a hint / the optimisation tool), e.g. "PNG → WebP" */
	advice?: string;
	/** a conversion exists here (else only the advice) */
	convertible: boolean;
}

const IMAGE_CONVERT = /^image\/(png|jpeg)$/;

export function mediaKind(file: Pick<File, 'type' | 'name'>): MediaKind {
	if (file.type.startsWith('image/') || /\.(png|jpe?g|webp|svg|gif)$/i.test(file.name)) return 'image';
	if (file.type.startsWith('video/')) return 'video';
	if (file.type.startsWith('audio/')) return 'audio';
	return 'other';
}

/** does the file meet the standard – and can it be converted here? */
export function checkMedia(file: Pick<File, 'type' | 'name'>): MediaVerdict {
	const kind = mediaKind(file);
	if (kind === 'image') {
		if (IMAGE_CONVERT.test(file.type)) return { ok: false, advice: `${file.type === 'image/png' ? 'PNG' : 'JPEG'} → WebP`, convertible: true };
		return { ok: true, convertible: false };
	}
	if (kind === 'video') {
		// MP4 / WebM are the standard; other containers would need ffmpeg (not on the server – later, BACKLOG 10.64)
		const ok = /^video\/(mp4|webm)$/.test(file.type);
		return ok ? { ok, convertible: false } : { ok, advice: '→ MP4 (H.264) / WebM', convertible: false };
	}
	return { ok: true, convertible: false };
}

/** the standard file name for a converted file (`a.png` → `a.webp`); unchanged when nothing converts */
export function standardName(name: string, file: Pick<File, 'type' | 'name'> = { type: '', name }): string {
	return checkMedia(file).convertible && mediaKind(file) === 'image' ? name.replace(/\.(png|jpe?g)$/i, '.webp') : name;
}

/**
 * The file in the standard: a PNG / JPEG as WebP at full size (`maxWidth` scales down), anything else as it is (video:
 * not yet – see above). Returns the original when nothing converts.
 */
export async function toStandard(file: File, options: { maxWidth?: number } = {}): Promise<File> {
	const verdict = checkMedia(file);
	if (!verdict.convertible) return file;
	const bitmap = await createImageBitmap(file);
	try {
		const { blob } = await toWebp(bitmap, options.maxWidth ?? bitmap.width);
		return new File([blob], standardName(file.name, file), { type: 'image/webp' });
	} finally {
		bitmap.close();
	}
}
