/**
 * Template utilities for safe HTML rendering (pure, no framework / bundler globals)
 */
import type { Season, SequenceItem } from './content.ts';
import { Status } from './enums.ts';

export function escapeHtml(text: string): string {
	if (text === null || text === undefined) {
		return '';
	}

	const str = String(text);
	const htmlEscapeMap = {
		'&': '&amp;',
		'<': '&lt;',
		'>': '&gt;',
		'"': '&quot;',
		"'": '&#x27;',
		'/': '&#x2F;'
	};

	return str.replace(/[&<>"'/]/g, (char) => htmlEscapeMap[char as keyof typeof htmlEscapeMap]);
}

export function sanitizeUrl(url: string) {
	if (!url) return '';

	const urlStr = String(url).trim().toLowerCase();

	// Block dangerous protocols
	const dangerousProtocols = ['javascript:', 'data:', 'vbscript:', 'file:'];
	for (const protocol of dangerousProtocols) {
		if (urlStr.startsWith(protocol)) {
			console.warn('Blocked dangerous URL protocol:', url);
			return '';
		}
	}

	return url;
}

export function escapeAndJoin(arr: string[], separator = ', ') {
	if (!Array.isArray(arr)) return '';
	return arr.map((item) => escapeHtml(item)).join(separator);
}

export function sanitizeNumber(value: number | string, defaultValue = 1, min = 1, max = 999) {
	if (typeof value === 'number') value = value.toString();
	const num = parseInt(value, 10);
	if (isNaN(num) || num < min || num > max) {
		return defaultValue;
	}
	return num;
}

/**
 * Whether a season sequence item can be read: `published` (or no status) always, `draft` only when `includeDrafts`
 * (the viewer passes `import.meta.env.DEV`), `coming-soon` never. Pass the season's item (`Season.sequence[]`, per
 * locale) – the chapter file's own `status` is deprecated and not read.
 */
export function isItemPublic(
	item: SequenceItem | { status?: string } | null | undefined,
	includeDrafts = false
) {
	if (!item) return false;
	const status = item.status || 'published';
	return status === 'published' || (status === 'draft' && includeDrafts);
}

/** Whether a season sequence item is listed in the chapter overview: readable ones and `coming-soon` (not clickable). */
export function isItemListed(item: SequenceItem | { status?: string } | null | undefined, includeDrafts = false) {
	return isItemPublic(item, includeDrafts) || item?.status === Status.COMING_SOON;
}

/** The season status of chapter `number` (the source of its visibility), or null when the sequence doesn't have it. */
export function sequenceStatus(season: Pick<Season, 'sequence'> | null | undefined, number: number): Status | null {
	const item = (season?.sequence ?? []).find((i) => Number(i?.number) === Number(number));
	return item?.status ?? null;
}

/** Absolute URL of a file on the CDN (`base` without trailing slash); '' for empty or unsafe paths. */
export function cdnUrl(base: string, path: string): string {
	if (!path || typeof path !== 'string') {
		return '';
	}

	const safePath = sanitizeUrl(path);
	if (!safePath) {
		return '';
	}

	return `${base}/${safePath.replace(/^\/+/, '')}`;
}
