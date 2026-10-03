import { isItemListed, isItemPublic, Status } from '@scutoons/content';

/**
 * Draft preview (owner 2026-10-03: "a flag for authorized creators to view chapters that are also in draft or coming
 * soon mode for debugging"). A per-device switch on the account page, offered to the role creator only. While it is on,
 * the library is read with `?draft=1` (the API checks the role: newest versions, no status gates) and the reader's own
 * gates let draft and coming-soon chapters (and draft toons) through. A banner says so on every page.
 * In a dev build drafts are readable anyway (as before); coming-soon chapters still need the preview.
 */
const KEY = 'scutoons.draft-preview';

function stored(): boolean {
	try {
		return typeof localStorage !== 'undefined' && localStorage.getItem(KEY) === '1';
	} catch {
		return false;
	}
}

let on = $state(stored());

export const draftPreview = {
	get on() {
		return on;
	},
	set(value: boolean) {
		on = value;
		try {
			if (value) localStorage.setItem(KEY, '1');
			else localStorage.removeItem(KEY);
		} catch {
			// storage blocked: the switch lasts for this page view
		}
	}
};

/** drafts are readable: a dev build or a creator's draft preview */
export const showDrafts = (): boolean => import.meta.env.DEV || on;

/** a chapter / toon the reader opens: published; drafts with showDrafts(); coming soon only in the preview */
export function isReadable(item: { status?: string } | null | undefined): boolean {
	return isItemPublic(item, showDrafts()) || (on && item?.status === Status.COMING_SOON);
}

/** listed in the chapter list (coming soon always, drafts with showDrafts()) */
export const isListed = (item: { status?: string } | null | undefined): boolean =>
	isItemListed(item, showDrafts());
