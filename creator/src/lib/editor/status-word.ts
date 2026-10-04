import { Status } from '@scutoons/content';
import { m } from '$lib/paraglide/messages.js';

/** A chapter status as a plain word ("published", "draft", "coming soon") in the UI language. */
export function statusWord(status: string): string {
	if (status === Status.PUBLISHED) return m['creator.status_published']();
	if (status === Status.COMING_SOON) return m['creator.status_coming_soon']();
	if (status === Status.DRAFT) return m['creator.status_draft']();
	return status;
}
