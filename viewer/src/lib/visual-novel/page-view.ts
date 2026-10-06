import type { PageView } from '@scutoons/content';

/**
 * The page view the reader is on (agents/EXPRESSIONS.md §5, recurrence.ts): one stay on a page – its path and its
 * `?label=` (a group of a chain). Another page, another label, or a reload starts a new one. The repeat rule "once per
 * page view" of events and a placed item's respawn "every visit" both count it. The toon layout calls `pageView()`
 * after every navigation, so going away and coming back is a new view even when nothing on the other page asked.
 */
let current: PageView & { key: string } = { key: '', id: '', start: 0 };

export function pageView(): PageView {
	const key = typeof location === 'undefined' ? '' : `${location.pathname}?${new URLSearchParams(location.search).get('label') ?? ''}`;
	if (current.key !== key || !current.id) current = { key, id: Math.random().toString(36).slice(2, 10), start: Date.now() };
	return current;
}
