// Labels of the API's vocabularies (track, stage, type, kind, outcome) and dates, from the paraglide messages.
import { m } from '$lib/paraglide/messages.js';
import { getLocale } from '$lib/paraglide/runtime.js';

const messages = m as unknown as Record<string, ((args?: Record<string, unknown>) => string) | undefined>;

/** `label('stage', 'in_conversation')` → "in conversation"; unknown values come back as they are */
export function label(group: 'track' | 'stage' | 'type' | 'kind' | 'outcome', value: string | null | undefined): string {
	if (!value) return '';
	return messages[`${group}.${value}`]?.() ?? value;
}

/** Today in the browser's time zone, YYYY-MM-DD */
export function todayIso(): string {
	const d = new Date();
	return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

export function addDays(iso: string, days: number): string {
	const d = new Date(`${iso}T12:00:00`);
	d.setDate(d.getDate() + days);
	return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

/** Days from today to a YYYY-MM-DD date (negative = in the past) */
export function daysFromToday(iso: string): number {
	return Math.round((new Date(`${iso}T12:00:00`).getTime() - new Date(`${todayIso()}T12:00:00`).getTime()) / 86400000);
}

/** "today", "tomorrow", "in 3 days", "3 days ago" */
export function relativeDay(iso: string | null | undefined): string {
	if (!iso) return '';
	const days = daysFromToday(iso.slice(0, 10));
	if (days === 0) return m['date.today']();
	if (days === 1) return m['date.tomorrow']();
	return days > 1 ? m['date.in_days']({ days }) : m['date.days_ago']({ days: -days });
}

/** "Mon 6 Oct" in the UI language */
export function shortDate(iso: string | null | undefined): string {
	if (!iso) return '';
	const d = new Date(`${iso.slice(0, 10)}T12:00:00`);
	return d.toLocaleDateString(getLocale(), { weekday: 'short', day: 'numeric', month: 'short' });
}

/** A UTC `YYYY-MM-DD HH:MM:SS` from the API as a local date + time */
export function dateTime(utc: string): string {
	const d = new Date(`${utc.replace(' ', 'T')}Z`);
	return d.toLocaleString(getLocale(), { day: 'numeric', month: 'short', year: 'numeric', hour: '2-digit', minute: '2-digit' });
}

/** Comma/semicolon/newline separated text → list */
export function splitList(text: string): string[] {
	return text
		.split(/[,;\n]/)
		.map((s) => s.trim())
		.filter(Boolean);
}

/** tel: link of a phone number as written */
export const telHref = (phone: string) => `tel:${phone.replace(/[^\d+]/g, '')}`;

/** Why an item is in someone's weekly portion (the API's `hint`, agents/EMBASSY.md "Pace"), in the app's language */
export function hintText(hint: { kind: string; days?: number; subject?: string; date?: string }): string {
	const days = hint.days ?? 0;
	switch (hint.kind) {
		case 'mail_no_answer':
			return hint.subject ? m['hint.mail_no_answer']({ days, subject: hint.subject }) : m['hint.mail_no_answer_plain']({ days });
		case 'voicemail':
			return m['hint.voicemail']({ days });
		case 'no_answer':
			return m['hint.no_answer']({ days });
		case 'replied':
			return hint.subject ? m['hint.replied']({ days, subject: hint.subject }) : m['hint.replied_plain']({ days });
		case 'call_back':
			return m['hint.call_back']({ date: shortDate(hint.date) });
		case 'answered_then_quiet':
			return m['hint.answered_then_quiet']({ date: shortDate(hint.date) });
		case 'no_contact_yet':
			return m['hint.no_contact_yet']();
		default:
			return hint.date ? m['hint.due_since']({ date: shortDate(hint.date) }) : m['hint.due']();
	}
}
