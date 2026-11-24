// What the main process decides without Electron: the schemes from the one link definition, a link among a start's
// arguments, the configuration, and where a navigation may go. Pure: test/link.test.ts.

/** the two apps' schemes (server/schema/tables/deck-link.json: `scheme`, `creator.scheme`) */
export interface Schemes {
	creator: string;
	deck: string;
}

export function readSchemes(raw: unknown): Schemes {
	const table = raw as { scheme?: unknown; creator?: { scheme?: unknown } } | null;
	const deck = table?.scheme;
	const creator = table?.creator?.scheme;
	if (typeof deck !== 'string' || typeof creator !== 'string') throw new Error('deck-link.json: no scheme for the deck or the creator');
	return { creator, deck };
}

/** the first argument that is a link of `scheme` (Windows and Linux start a process with the link as an argument) */
export function linkIn(argv: readonly string[], scheme: string): string | null {
	const prefix = `${scheme.toLowerCase()}:`;
	return argv.find((arg) => arg.toLowerCase().startsWith(prefix)) ?? null;
}

export interface AppConfig {
	/** where the creator is loaded from: the deployed web creator, the same code */
	creatorUrl: string;
	/** the deck's download, offered when no deck is installed; null: none yet */
	deckDownloadUrl: string | null;
}

const web = (value: unknown): URL | null => {
	if (typeof value !== 'string' || !value.trim()) return null;
	try {
		const url = new URL(value.trim());
		return url.protocol === 'https:' || url.protocol === 'http:' ? url : null;
	} catch {
		return null;
	}
};

const LOOPBACK = new Set(['localhost', '127.0.0.1', '[::1]']);
/** https, or http on this machine only (a local stack) */
const safe = (url: URL | null): URL | null => (url && (url.protocol === 'https:' || LOOPBACK.has(url.hostname)) ? url : null);

/** config.json, the environment over it (SCUTOONS_CREATOR_URL, SCUTOONS_DECK_DOWNLOAD_URL) */
export function readConfig(raw: unknown, env: Readonly<Record<string, string | undefined>> = {}): AppConfig {
	const file = (raw ?? {}) as { creatorUrl?: unknown; deckDownloadUrl?: unknown };
	const creator = safe(web(env.SCUTOONS_CREATOR_URL)) ?? safe(web(file.creatorUrl));
	if (!creator) throw new Error('config.json: creatorUrl must be an https address (http only on this machine)');
	const download = safe(web(env.SCUTOONS_DECK_DOWNLOAD_URL)) ?? safe(web(file.deckDownloadUrl));
	return { creatorUrl: creator.href, deckDownloadUrl: download?.href ?? null };
}

/** the same origin as the creator: stays in the window */
export function sameOrigin(url: string, base: string): boolean {
	try {
		return new URL(url).origin === new URL(base).origin;
	} catch {
		return false;
	}
}

/** what leaves the window goes to the system: the web, mail and the deck's link; anything else is refused */
export function goesOutside(url: string, schemes: Schemes): boolean {
	try {
		const protocol = new URL(url).protocol;
		return protocol === 'https:' || protocol === 'http:' || protocol === 'mailto:' || protocol === `${schemes.deck}:`;
	} catch {
		return false;
	}
}
