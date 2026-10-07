// Labels of the newsletter pages (agents/NEWSLETTER.md): status words + badge tones, audience names, error codes of
// the API that the pages show in the admin's language instead of the API's English message.
import { ApiError, type NewsletterAudience, type NewsletterStatus } from '@scutoons/api';
import { errorText } from '$lib/format.ts';
import { m } from '$lib/paraglide/messages.js';

export const newsletterStatus = (status: NewsletterStatus) =>
	({
		draft: m['newsletter.status_draft'](),
		sending: m['newsletter.status_sending'](),
		paused: m['newsletter.status_paused'](),
		done: m['newsletter.status_done']()
	})[status] ?? status;

const TONES = { draft: 'draft', sending: 'info', paused: 'warning', done: 'neutral' } as const;

export const statusTone = (status: NewsletterStatus) => TONES[status] ?? 'neutral';

export function audienceLabel(audience: Pick<NewsletterAudience, 'id' | 'source'>): string {
	if (audience.id === 'members_opted_in') return m['newsletter.audience_members_opted_in']();
	if (audience.id === 'members_all') return m['newsletter.audience_members_all']();
	if (audience.id === 'contacts') return m['newsletter.audience_contacts']();
	return m['newsletter.audience_source']({ source: audience.source ?? audience.id.replace(/^contacts:/, '') });
}

/** An API error in the admin's language where the code is known */
export function newsletterError(error: unknown): string {
	if (!(error instanceof ApiError)) return errorText(error);
	const body = error.body ?? {};
	switch (error.code) {
		case 'missing_locale':
			return m['newsletter.missing_locale']({ count: Number(body.count ?? 0) });
		case 'missing_text':
			return m['newsletter.missing_text']({ locale: String(body.locale ?? '') });
		case 'no_recipients':
			return m['newsletter.no_recipients']();
		case 'no_audience':
			return m['newsletter.no_audience']();
		default:
			return error.message;
	}
}
