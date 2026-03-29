/**
 * Template utilities for safe HTML rendering
 */
const isDev = import.meta.env.DEV;

import type { Chapter } from '$lib/models/content.ts';

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

	return str.replace(/[&<>"'/]/g, (char) => htmlEscapeMap[char]);
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

export function isItemPublic(item: Chapter) {
	if (!item) return false;
	const status = item.status || 'published';
	return status === 'published' || (status === 'draft' && isDev);
}
