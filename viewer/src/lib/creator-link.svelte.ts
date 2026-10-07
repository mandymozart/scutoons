import { goto } from '$app/navigation';
import { isReaderReady, parseReaderShow, READER_READY, type ReaderShow } from '@scutoons/content';
import { CREATOR_URL } from '$lib/config.ts';
import { draftPreview } from '$lib/draft-preview.svelte.ts';
import { deLocalizeHref, localizeHref } from '$lib/paraglide/runtime.js';

/**
 * The creator's dock Reader (BACKLOG 10.49, agents/DOCKING.md "The Reader panel"): the creator frames this app's draft
 * preview (`?draft=1`) and asks it to show a group / slice (`@scutoons/content` reader-link.ts). Accepted ONLY when all
 * hold: this page is framed, the message comes from the frame's parent window, from the creator's configured origin
 * (`CREATOR_URL`: dev http://localhost:10000, production https://creator.scutoons.com), the draft preview is on, and the
 * message is well formed. Anything else is ignored. A request only moves the reader (another chapter: an in-app
 * navigation, then the chapter page scrolls – its +page.svelte); it never reads or changes data. Nothing goes back but
 * "ready", and only to the creator's origin.
 */
const CREATOR_ORIGIN = new URL(CREATOR_URL).origin;

export interface ShowRequest extends ReaderShow {
	/** increases with every accepted request (the chapter page handles each once) */
	seq: number;
}

let request = $state<ShowRequest | null>(null);
let accepted = 0;

export const creatorLink = {
	/** the newest accepted "show" request */
	get request() {
		return request;
	}
};

const framed = () => typeof window !== 'undefined' && window.parent !== window;

/** the message passes every check (origin, source window, framed, draft preview) */
export function fromCreator(event: MessageEvent): boolean {
	return framed() && event.origin === CREATOR_ORIGIN && event.source === window.parent && draftPreview.on;
}

/**
 * Listens to the creator (the root layout, on mount); framed with the draft preview on, it says "ready" to the
 * creator's origin. Not framed: nothing at all.
 */
export function listenToCreator(): () => void {
	if (!framed()) return () => {};
	const onmessage = (event: MessageEvent) => {
		if (!fromCreator(event) || isReaderReady(event.data)) return;
		const show = parseReaderShow(event.data);
		if (!show) return;
		request = { ...show, seq: ++accepted };
		// the count of accepted requests on <html> (the headless check proves a foreign one is ignored)
		document.documentElement.dataset.creatorShows = String(accepted);
		const path = `/${show.toon}/chapter/${show.chapter}`;
		if (deLocalizeHref(location.pathname).replace(/\/$/, '') !== path) void goto(localizeHref(path), { replaceState: true });
	};
	window.addEventListener('message', onmessage);
	if (draftPreview.on) window.parent.postMessage({ type: READER_READY }, CREATOR_ORIGIN);
	return () => window.removeEventListener('message', onmessage);
}
