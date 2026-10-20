// The personal sign-in link of a newsletter's `{signin}` button (#65, agents/NEWSLETTER.md "Writing a newsletter"):
// `/?signin=<token>`. Mail scanners open links, so the page asks once before it signs in (the token works once). A
// QR contact signs up with it – the account comes with its code; a code that offers a choice opens its claim sheet.
// A gift button's link (#66) carries `&gift=<token>` too: once signed in – or when the sign-in is declined – the gift
// sheet opens (`then`). A sign-in button with a target (#106) carries `&to=<path>`: once signed in – or declined – the
// reader goes there (`then` as well, see the root layout); the failure dialog's sign-in comes back to it.
import { confirmDialog } from '@scutoons/ui';
import { goto } from '$app/navigation';
import { ApiError } from '$lib/api.ts';
import { localizeHref } from '$lib/paraglide/runtime.js';
import { m } from '$lib/paraglide/messages.js';
import type { SessionContext } from '$lib/contexts/session.svelte.ts';

export async function newsletterSignin(
	token: string,
	session: SessionContext,
	openClaim: (code: string, auto: boolean) => void,
	then?: () => void,
	to?: string | null
) {
	const yes = await confirmDialog({
		title: m['newsletter_signin.title'](),
		message: m['newsletter_signin.message'](),
		confirmLabel: m['newsletter_signin.confirm'](),
		cancelLabel: m['newsletter_signin.cancel']()
	});
	if (!yes) return then?.();
	try {
		const result = await session.verifyToken(token);
		if (then) return then();
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
		if (again) goto(localizeHref('/account') + (to ? '?return=' + encodeURIComponent(localizeHref(to)) : ''));
		else then?.();
	}
}
