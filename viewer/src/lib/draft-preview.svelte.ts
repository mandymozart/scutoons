import { isItemListed, isItemPublic, Status } from '@scutoons/content';
import { linkedPreview } from '$lib/draft-preview-link.ts';

/**
 * Draft preview (owner 2026-10-03: "a flag for authorized creators to view chapters that are also in draft or coming
 * soon mode for debugging"). A per-device switch on the account page, offered to the role creator only. While it is on,
 * the library is read with `?draft=1` (the API checks the role: newest versions, no status gates) and the reader's own
 * gates let draft and coming-soon chapters (and draft toons) through. A small eye mark (top right, root layout) says so
 * on every page – the switch itself is only on the account page.
 * In a dev build drafts are readable anyway (as before); coming-soon chapters still need the preview.
 * The creator's docked reader opens `?draft=1` (older links: `?preview=draft`, owner 2026-10-04: no own preview in the
 * creator): read here, when the module loads, so the preview is on before the first load function asks the library
 * (draft-preview-link.ts); the root layout takes `preview` out of the address.
 */
const KEY = 'scutoons.draft-preview';

function stored(): boolean {
	try {
		return typeof localStorage !== 'undefined' && localStorage.getItem(KEY) === '1';
	} catch {
		return false;
	}
}

/** what the address the app was opened with says (`?draft=1` / `?draft=0` / `?preview=draft`), null = nothing */
function linked(): boolean | null {
	try {
		return typeof location !== 'undefined' ? linkedPreview(location.search) : null;
	} catch {
		return null;
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

const link = linked();
if (link !== null) draftPreview.set(link);

/** drafts are readable: a dev build or a creator's draft preview */
export const showDrafts = (): boolean => import.meta.env.DEV || on;

/** a chapter / toon the reader opens: published; drafts with showDrafts(); coming soon only in the preview */
export function isReadable(item: { status?: string } | null | undefined): boolean {
	return isItemPublic(item, showDrafts()) || (on && item?.status === Status.COMING_SOON);
}

/**
 * saved to the device: published only, also in the draft preview – the server's file list refuses drafts and coming
 * soon (403 not_public), and one refused list failed the whole season as "the server didn't deliver a file" (#194)
 */
export const isSavable = (item: { status?: string } | null | undefined): boolean => isItemPublic(item);

/** listed in the chapter list (coming soon always, drafts with showDrafts()) */
export const isListed = (item: { status?: string } | null | undefined): boolean =>
	isItemListed(item, showDrafts());
