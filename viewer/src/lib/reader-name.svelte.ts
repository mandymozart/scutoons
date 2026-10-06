// The reader's name (agents/EXPRESSIONS.md §6): what an askText with scope "reader" asked – `{reader.name}` in every
// toon's texts. Members: on the account (`/me/ user.display_name`, `PUT /me/ { display_name }` – every device), guests:
// on this device (localStorage `scutoons_reader_name`, one key for all of SCUTOONS). Plain text, trimmed, at most 32
// characters (the server's users.display_name). Never shown to anyone else.
import { browser } from '$app/environment';
import { getCurrentSession } from '$lib/contexts/session.svelte.ts';

export const READER_NAME_KEY = 'scutoons_reader_name';
/** the server's limit (users.display_name) */
export const READER_NAME_MAX = 32;

/** a typed answer as it is stored: control characters / line breaks → spaces, runs of white space one, trimmed, cut */
export function cleanText(raw: unknown, max: number): string {
	if (typeof raw !== 'string') return '';
	return [...raw.replace(/[\p{C}\s]+/gu, ' ').trim()].slice(0, max).join('').trim();
}

function readLocal(): string {
	try {
		return cleanText(localStorage.getItem(READER_NAME_KEY) ?? '', READER_NAME_MAX);
	} catch {
		return '';
	}
}

class ReaderNameState {
	/** this device's name (guests) */
	local = $state<string>(browser ? readLocal() : '');

	constructor() {
		if (!browser) return;
		// another tab changed it
		window.addEventListener('storage', (e) => {
			if (e.key === READER_NAME_KEY) this.local = readLocal();
		});
	}

	/** the reader's name: a member's (account) – else this device's; undefined = none */
	get value(): string | undefined {
		const session = getCurrentSession();
		if (session?.isLoggedIn) return session.user?.display_name || undefined;
		return this.local || undefined;
	}

	/**
	 * Stores the name ("" clears it): a member's on the account, a guest's on this device – decided once it is known who
	 * reads (a member typing while the session check still runs must not land on the device).
	 */
	async set(raw: string): Promise<void> {
		const name = cleanText(raw, READER_NAME_MAX);
		await readerKnown();
		const session = getCurrentSession();
		if (session?.isLoggedIn) {
			await session.setDisplayName(name || null);
			return;
		}
		this.local = name;
		try {
			if (name) localStorage.setItem(READER_NAME_KEY, name);
			else localStorage.removeItem(READER_NAME_KEY);
		} catch {
			// private mode: the name lasts for this page
		}
	}
}

export const readerName = new ReaderNameState();

/**
 * Resolves once it is known who reads (the session check of a signed-in browser is done – at most `ms`): a script's
 * texts and conditions read `reader.name`, `reader.member` … from the start (else a member's name, still loading, would
 * be asked for again).
 */
export async function readerKnown(ms = 3000): Promise<void> {
	const end = Date.now() + ms;
	while (getCurrentSession()?.pending && Date.now() < end) await new Promise((r) => setTimeout(r, 50));
}

