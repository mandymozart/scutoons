/**
 * The Info view's source (deck-kit editor/info.rs, from the OSCT editor; owner 2026-11-24, RULES H14): the name and the
 * description of what is under the pointer. The screen carries no hint lines or info buttons – what a control does is
 * its hover text (`title`), and a member who wants it opens the Info view. Only read while the view is open.
 */
import { shared } from '$lib/utils/shared.ts';

export interface InfoText {
	name: string;
	text: string;
}

/** what an element under the pointer says about itself: its name (aria-label or its words) and its hover text */
export function infoOf(target: EventTarget | null): InfoText | null {
	const el = target instanceof Element ? target.closest<HTMLElement>('[title], [aria-label]') : null;
	if (!el) return null;
	const title = el.getAttribute('title')?.trim() ?? '';
	const name = (el.getAttribute('aria-label') ?? el.textContent ?? '').trim().replace(/\s+/g, ' ').slice(0, 80);
	if (!title && !name) return null;
	return title && title !== name ? { name, text: title } : { name: name || title, text: '' };
}

class Info {
	/** the last thing the pointer rested on; it stays while the pointer is over nothing that speaks */
	shown = $state<InfoText | null>(null);
}

export const info = shared('creator-info', () => new Info());
