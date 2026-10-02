import { getLocale } from '$lib/paraglide/runtime.js';

/** `YYYY-MM-DD` of a date string (server-local or ISO), '—' when empty */
export const day = (value: string | null | undefined) => (value ? String(value).slice(0, 10) : '—');

/** `YYYY-MM-DD HH:MM` */
export const minute = (value: string | null | undefined) => (value ? String(value).replace('T', ' ').slice(0, 16) : '—');

export const number = (value: number | null | undefined) =>
	value === null || value === undefined ? '—' : new Intl.NumberFormat(getLocale()).format(value);

export const percent = (part: number, total: number) =>
	total > 0 ? new Intl.NumberFormat(getLocale(), { style: 'percent', maximumFractionDigits: 0 }).format(part / total) : '—';

/** `2026-09-30` → `30.09.` / `09/30` style short label for chart axes */
export const shortDay = (iso: string) => {
	const [, mm, dd] = iso.split('-');
	return getLocale() === 'de' ? `${dd}.${mm}.` : `${mm}/${dd}`;
};

/** Message of an unknown error */
export const errorText = (error: unknown) => (error instanceof Error ? error.message : String(error));

/** 1234567 → "1.2 MB" (binary units) */
export function bytes(value: number | null | undefined): string {
	if (value === null || value === undefined) return '—';
	if (value < 1024) return `${value} B`;
	const units = ['KB', 'MB', 'GB', 'TB'];
	let v = value / 1024;
	let i = 0;
	while (v >= 1024 && i < units.length - 1) {
		v /= 1024;
		i++;
	}
	return `${new Intl.NumberFormat(getLocale(), { maximumFractionDigits: v < 10 ? 1 : 0 }).format(v)} ${units[i]}`;
}
