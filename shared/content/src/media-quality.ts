// Media quality (owner 2026-10-01): a reader setting, not a responsive question. Three levels + "Automatic":
//   low / medium / high ("Niedrig" / "Mittel" / "Hoch"), `auto` = by the measured bandwidth (autoQuality).
// Members store it on the server (`users.media_quality`, `GET/PUT /me/`), guests in localStorage
// (`scutoons_media_quality`). Media with several files carry `variants` keyed by the level next to their default `src`;
// `pickQualityVariant` turns a level into a file. Implemented for slice videos (480p / 720p / 1080p); the same levels and the
// same `{ src, variants }` shape is used by audio (`AudioTrack { path, variants }`, 96k / 128k / 320k, 2026-10-04) and is
// meant for slice images (1x / 2x / 3x WebP) later (agents/DESIGN.md "Media quality").

/** The quality levels, lowest first. */
export const MEDIA_QUALITIES = ['low', 'medium', 'high'] as const;
export type MediaQuality = (typeof MEDIA_QUALITIES)[number];

/** What a reader can choose: a level or `auto` (by the measured bandwidth). */
export const MEDIA_QUALITY_SETTINGS = ['auto', ...MEDIA_QUALITIES] as const;
export type MediaQualitySetting = (typeof MEDIA_QUALITY_SETTINGS)[number];

/** Every reader starts on Automatic (members: the column default; guests: nothing stored). */
export const MEDIA_QUALITY_DEFAULT: MediaQualitySetting = 'auto';

/** One file per quality level (CDN paths like `src`); a missing level falls back (`pickQualityVariant`). */
export type QualityVariants = Partial<Record<MediaQuality, string>>;

/** What each level means for a video (the encodings on the CDN: `<name>.480p.mp4` …). */
export const VIDEO_QUALITY_LABELS: Record<MediaQuality, string> = { low: '480p', medium: '720p', high: '1080p' };

/**
 * What each level means for audio (music and samples, owner 2026-10-04): `<name>.96.mp3` / `.128.mp3` / `.320.mp3`, the
 * kbit/s in the name (the asset registry groups them: ≤ 96 low, ≤ 160 medium, above high – AssetPaths).
 */
export const AUDIO_QUALITY_KBPS: Record<MediaQuality, number> = { low: 96, medium: 128, high: 320 };
export const AUDIO_QUALITY_LABELS: Record<MediaQuality, string> = { low: '96k', medium: '128k', high: '320k' };

/** Later, for slice images: the density per level (1x / 2x / 3x WebP, 390 / 780 / 1170 px wide). Not used yet. */
export const IMAGE_QUALITY_DENSITIES: Record<MediaQuality, number> = { low: 1, medium: 2, high: 3 };

export function isMediaQuality(value: unknown): value is MediaQuality {
	return typeof value === 'string' && (MEDIA_QUALITIES as readonly string[]).includes(value);
}

export function isMediaQualitySetting(value: unknown): value is MediaQualitySetting {
	return typeof value === 'string' && (MEDIA_QUALITY_SETTINGS as readonly string[]).includes(value);
}

/**
 * The file for a level: the level's variant, else the nearest lower one, else the default `src` (a reader on "low"
 * never gets a bigger file than the author's default because a variant is missing).
 */
export function pickQualityVariant(media: { src: string; variants?: QualityVariants | null }, quality: MediaQuality): string {
	const variants = media.variants ?? {};
	for (let i = MEDIA_QUALITIES.indexOf(quality); i >= 0; i--) {
		const file = variants[MEDIA_QUALITIES[i]];
		if (typeof file === 'string' && file) return file;
	}
	return media.src;
}

/** The level a media plays at with the default `src` (when `src` is one of its variants), else null. */
export function variantLevel(media: { src: string; variants?: QualityVariants | null }, file = media.src): MediaQuality | null {
	for (const level of MEDIA_QUALITIES) if (media.variants?.[level] === file) return level;
	return null;
}

/**
 * Bandwidth (Mbit/s) a level needs on "Automatic": ≈ 1.5 × the bitrate of the chapter-0 encodings (480p 0.9, 720p 3.3,
 * 1080p 4.5 Mbit/s with sound). Below `medium` → low.
 */
export const AUTO_QUALITY_MBPS = { medium: 5, high: 7 } as const;

/** The level for a measured / estimated throughput in Mbit/s (conservative: rounding down). */
export function qualityForMbps(mbps: number): MediaQuality {
	if (!(mbps > 0)) return 'low';
	if (mbps >= AUTO_QUALITY_MBPS.high) return 'high';
	if (mbps >= AUTO_QUALITY_MBPS.medium) return 'medium';
	return 'low';
}

/** The bits of the Network Information API that "Automatic" reads (Chromium; Safari and Firefox have none). */
export interface NetworkHint {
	saveData?: boolean;
	effectiveType?: string;
	/** estimated downlink in Mbit/s (Chromium rounds it and caps it at 10) */
	downlink?: number;
}

/** Below this estimated downlink (Mbit/s) the Network Information API alone decides "low". */
export const AUTO_SLOW_DOWNLINK = 1;

/**
 * "Automatic" from the Network Information API: data saver, a 2G/3G-class connection or a downlink estimate under
 * 1 Mbit/s → low right away. Anything else → null = measure a download (`qualityForMbps`): Chromium's `downlink` is too
 * coarse to tell 720p from 1080p (a fresh profile reports ≈ 1.4 Mbit/s even on localhost, 2026-10-01), so it only ever
 * lowers the quality, never raises it.
 */
export function autoQuality(net: NetworkHint | null | undefined): MediaQuality | null {
	if (!net) return null;
	if (net.saveData) return 'low';
	if (net.effectiveType && net.effectiveType !== '4g') return 'low';
	if (typeof net.downlink === 'number' && net.downlink > 0 && net.downlink < AUTO_SLOW_DOWNLINK) return 'low';
	return null;
}

/** Resolves a setting: a level stays, `auto` becomes the given automatic level (unknown yet → low, conservative). */
export function effectiveQuality(setting: MediaQualitySetting, auto: MediaQuality | null): MediaQuality {
	return setting === 'auto' ? (auto ?? 'low') : setting;
}
