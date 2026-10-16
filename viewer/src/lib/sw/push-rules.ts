// Web Push in the worker (agents/research/PWA.md §2b) – pure: what a push message shows, where a tap goes, the badge.
// Tested in test/push-rules.test.ts; the handlers are lib/sw/push.ts.

/** the badge's state: the tags counted since the app was last opened (the worker adds, lib/app-badge.ts clears) */
export const BADGE_CACHE = 'scutoons-badge-v1';
export const BADGE_KEY = '/__scutoons/badge.json';

export interface PushNotice {
	title: string;
	body: string;
	/** a path on this origin the tap opens */
	url: string;
	/** one notification per tag on a device: a second send of the same chapter replaces the first */
	tag: string;
}

/** the server's payload (Domain\Push\NewChapterPush: { title, body, url, tag }) → what to show; anything odd → a plain one */
export function noticeOf(text: string | null | undefined): PushNotice {
	let data: Record<string, unknown> = {};
	try {
		const parsed = text ? JSON.parse(text) : null;
		if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) data = parsed as Record<string, unknown>;
	} catch {
		/* not JSON: the plain notice */
	}
	const str = (value: unknown, max: number) => (typeof value === 'string' ? value.trim().slice(0, max) : '');
	return {
		title: str(data.title, 120) || 'SCUTOONS',
		body: str(data.body, 240),
		url: safePath(data.url),
		tag: str(data.tag, 120) || 'scutoons'
	};
}

/** only a path on our own origin (never another site from a payload) */
export function safePath(value: unknown): string {
	if (typeof value !== 'string' || !value.startsWith('/') || value.startsWith('//') || value.includes('\\')) return '/';
	return value;
}

/**
 * The app icon's badge (owner proposal 2026-10-16, PWA.md §2b): the "new chapter" notifications that came since the
 * app was last opened – one more per push whose tag wasn't counted yet; opening the app clears it.
 */
export function nextBadge(stored: string | null | undefined, tag: string): { count: number; stored: string } {
	let tags: string[] = [];
	try {
		const parsed = stored ? JSON.parse(stored) : [];
		if (Array.isArray(parsed)) tags = parsed.filter((t): t is string => typeof t === 'string');
	} catch {
		/* broken: start again */
	}
	if (!tags.includes(tag)) tags.push(tag);
	tags = tags.slice(-99);
	return { count: tags.length, stored: JSON.stringify(tags) };
}
