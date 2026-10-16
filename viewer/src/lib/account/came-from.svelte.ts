// Where the reader came from before the account pages (owner 2026-10-06: from a chapter the account page needs a way
// back to it). Kept across `/account` ↔ `/account/<group>` (each is its own page component), so the index's back link
// still leads to the chapter after a visit to a group. `indexBehind`: the group page was opened from the index, so its
// back link is `history.back()` (the index's scroll position comes back) – else (a direct link) it goes to `/account`.
import { afterNavigate } from '$app/navigation';

const ACCOUNT_PATH = /\/account(\/|$)/;
const INDEX_PATH = /\/account\/?$/;

class CameFrom {
	url = $state<URL | null>(null);
	indexBehind = $state(false);
}

export const cameFrom = new CameFrom();

/** call once in each account page component */
export function trackCameFrom(): void {
	afterNavigate(({ from, to }) => {
		if (from?.url && !ACCOUNT_PATH.test(from.url.pathname)) cameFrom.url = from.url;
		cameFrom.indexBehind = !!from?.url && INDEX_PATH.test(from.url.pathname) && !!to?.url && !INDEX_PATH.test(to.url.pathname);
	});
}
