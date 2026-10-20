// The personal sign-in link of a newsletter's `{signin}` button (#65, agents/NEWSLETTER.md "Writing a newsletter"):
// `/?signin=<token>`. Mail scanners open links, so the page asks once before it signs in (the token works once). A
// QR contact signs up with it – the account comes with its code; a code that offers a choice opens its claim sheet.
import { confirmDialog } from '@scutoons/ui';
import { goto } from '$app/navigation';
import { ApiError } from '$lib/api.ts';
import { localizeHref } from '$lib/paraglide/runtime.js';
import { m } from '$lib/paraglide/messages.js';
import type { SessionContext } from '$lib/contexts/session.svelte.ts';

export async function newsletterSignin(token: string, session: SessionContext, openClaim: (code: string, auto: boolean) => void) {
	const yes = await confirmDialog({
		title: m['newsletter_signin.title'](),
		message: m['newsletter_signin.message'](),
		confirmLabel: m['newsletter_signin.confirm'](),
		cancelLabel: m['newsletter_signin.cancel']()
	});
	if (!yes) return;
	try {
		const result = await session.verifyToken(token);
		if (result.created) goto(localizeHref('/account'));
	} catch (e) {
		if (e instanceof ApiError && e.code === 'choose_item' && typeof e.body?.code === 'string') {
			openClaim(e.body.code, true);
			return;
		}
		const again = await confirmDialog({
			title: m['newsletter_signin.failed_title'](),
			message: m['newsletter_signin.failed'](),
			confirmLabel: m['newsletter_signin.to_login'](),
			cancelLabel: m['newsletter_signin.cancel']()
		});
		if (again) goto(localizeHref('/account'));
	}
}
