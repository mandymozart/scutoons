// Audio quality levels in the browser (owner 2026-10-04: "importing audio should handle this and create if possible the
// lower versions"): an upload or an existing audio asset gets its missing levels – low 96k / medium 128k / high 320k MP3,
// named `<name>.<kbps>.mp3` (the asset registry groups them into one asset: AssetPaths ≤ 96 low, ≤ 160 medium, above
// high). Only levels BELOW the source are made (a 128k file never becomes a fake 320k); a lossless source (WAV, FLAC,
// AIFF) gives all three and is not uploaded itself. The MP3 encoder (LAME as WebAssembly, `wasm-media-encoders`) loads
// only when audio is encoded. Runs where the creator / admin runs – no ffmpeg needed (the server's asset jobs need one).
import { AUDIO_QUALITY_KBPS, MEDIA_QUALITIES, type MediaQuality } from '@scutoons/content';

/** The CBR bit rates a level name may carry (LAME's), for naming an uploaded lossy file by its real rate. */
const STANDARD_KBPS = [64, 80, 96, 112, 128, 160, 192, 224, 256, 320] as const;
const SAMPLE_RATE = 44100;
/** A level is only made when it is clearly below the source (a 128k source makes 96k, not another 128k). */
const BELOW = 0.9;

export const LOSSLESS_AUDIO = /\.(wav|wave|flac|aif|aiff)$/i;

/** what an upload of `fileName` turns into */
export interface AudioUploadPlan {
	/** the path without marker and extension: catpope/s1/c1/s1c1-bells */
	stem: string;
	/** the uploaded file's own path (with its rate as the marker), null for a lossless source (not uploaded) */
	original: string | null;
	/** the source's bit rate (kbit/s, estimated from size / duration), null for lossless */
	sourceKbps: number | null;
	/** the levels to encode */
	levels: { level: MediaQuality; kbps: number; path: string }[];
}

/** "catpope/s1/c1/s1c1-bells.320.mp3" → "catpope/s1/c1/s1c1-bells" */
export function audioStem(path: string): string {
	return path.replace(/\.[a-z0-9]+$/i, '').replace(/\.\d{2,3}$/, '');
}

/** the nearest standard rate (a 317 kbit/s estimate of a CBR 320 file → 320) */
export function nearestKbps(kbps: number): number {
	return STANDARD_KBPS.reduce((best, k) => (Math.abs(k - kbps) < Math.abs(best - kbps) ? k : best), STANDARD_KBPS[0]);
}

/**
 * The plan for an upload at `path` (the picker's path field): the original keeps its format, its rate in the name; the
 * levels below it are encoded. `existing` = levels the asset already has (kept, not encoded again).
 */
export function planAudioUpload(path: string, size: number, duration: number, existing: Partial<Record<MediaQuality, string>> = {}): AudioUploadPlan {
	const stem = audioStem(path);
	const lossless = LOSSLESS_AUDIO.test(path);
	const sourceKbps = lossless || !(duration > 0) ? null : nearestKbps((size * 8) / duration / 1000);
	const ext = (path.match(/\.([a-z0-9]+)$/i)?.[1] ?? 'mp3').toLowerCase();
	const original = lossless ? null : sourceKbps ? `${stem}.${sourceKbps}.${ext}` : path;
	const levels = MEDIA_QUALITIES.filter((level) => !existing[level])
		.map((level) => ({ level, kbps: AUDIO_QUALITY_KBPS[level], path: `${stem}.${AUDIO_QUALITY_KBPS[level]}.mp3` }))
		.filter((l) => sourceKbps === null || l.kbps < sourceKbps * BELOW)
		// the original already is a level (e.g. a 128k upload is the medium file)
		.filter((l) => l.path !== original);
	return { stem, original, sourceKbps, levels };
}

/** the audio as PCM at 44.1 kHz (the browser decodes every format it can play) */
export async function decodeAudio(bytes: ArrayBuffer): Promise<AudioBuffer> {
	const ctx = new OfflineAudioContext({ numberOfChannels: 2, length: 1, sampleRate: SAMPLE_RATE });
	return ctx.decodeAudioData(bytes);
}

type Mp3Encoder = Awaited<ReturnType<typeof import('wasm-media-encoders').createMp3Encoder>>;
let encoderModule: Promise<typeof import('wasm-media-encoders')> | null = null;
async function mp3Encoder(): Promise<Mp3Encoder> {
	encoderModule ??= import('wasm-media-encoders');
	return (await encoderModule).createMp3Encoder();
}

/** CBR MP3 at `kbps` (one second per step, yielding to the page in between); `onprogress` gets 0…1 */
export async function encodeMp3(audio: AudioBuffer, kbps: number, onprogress?: (share: number) => void): Promise<Blob> {
	const encoder = await mp3Encoder();
	const channels = audio.numberOfChannels > 1 ? 2 : 1;
	encoder.configure({ channels, sampleRate: audio.sampleRate, bitrate: kbps as 96 });
	const data = Array.from({ length: channels }, (_, c) => audio.getChannelData(c));
	const parts: Uint8Array[] = [];
	const step = audio.sampleRate;
	for (let at = 0; at < audio.length; at += step) {
		// encode() returns a view into the encoder's memory: copy it before the next call
		parts.push(encoder.encode(data.map((d) => d.subarray(at, Math.min(at + step, audio.length)))).slice());
		if ((at / step) % 10 === 9) {
			onprogress?.(at / audio.length);
			await new Promise((resolve) => setTimeout(resolve, 0));
		}
	}
	parts.push(encoder.finalize().slice());
	onprogress?.(1);
	return new Blob(parts as BlobPart[], { type: 'audio/mpeg' });
}

/** The highest-quality file of an asset's renditions: a lossless original first, else the highest rate. */
export function bestAudioSource<T extends { path: string; label: string; mime?: string | null }>(renditions: T[]): T | null {
	const audio = renditions.filter((r) => r.mime?.startsWith('audio/') || /\.(mp3|m4a|aac|ogg|opus|wav|flac|aiff?)$/i.test(r.path));
	const lossless = audio.find((r) => LOSSLESS_AUDIO.test(r.path));
	if (lossless) return lossless;
	const rate = (r: T) => Number(r.label.match(/^(\d+)k$/)?.[1] ?? 0) || 1000; // a plain file (no marker) counts as the source
	return audio.sort((a, b) => rate(b) - rate(a))[0] ?? null;
}
