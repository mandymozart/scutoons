// Pairing a cyberdeck on the account page (#180, agents/research/CYBERDECKS.md §9; server docs/API.md "Decks"): the
// code on the deck's splash (6 characters, no 0 / O / 1 / I / L), typed in any case with or without the dash, or
// prefilled from the splash's QR code (`/account/decks?deck=<code>`), and the approval's lifetime (like the AI
// connections: six months by default, or shorter; optionally ending when the deck is unused for a while).

/** a paired deck (GET /me/decks) */
export interface Deck {
	id: string;
	name: string;
	hostname: string;
	build: string;
	class: string;
	last_seen_at: string | null;
	expires_at: string;
	idle_days: number | null;
	expired: boolean;
}

/** what waits behind a code (GET /me/decks/pending) */
export interface PendingDeck {
	code: string;
	hostname: string;
	build: string;
	class: string;
}

/** the server's alphabet (Domain\Decks\Decks::ALPHABET) */
const ALPHABET = /^[ABCDEFGHJKMNPQRSTUVWXYZ23456789]{6}$/;

/** the typed or scanned code → its 6 characters, or null when it can't be one */
export function normalizeCode(input: string | null | undefined): string | null {
	const code = (input ?? '').replace(/[^A-Za-z0-9]/g, '').toUpperCase();
	return ALPHABET.test(code) ? code : null;
}

/** shown like the splash: K7Q-42M (anything else as typed) */
export function formatCode(input: string): string {
	const code = normalizeCode(input);
	return code ? `${code.slice(0, 3)}-${code.slice(3)}` : input;
}

/** the approval's lifetime choices in days (the first is the default, the server's maximum) */
export const LIFETIME_DAYS = [182, 30, 7, 1] as const;
/** "ends when unused for …" in days; 0 = never */
export const IDLE_DAYS = [0, 30, 7, 1] as const;

export interface ApproveBody {
	code: string;
	days: number;
	idle_days?: number;
	name?: string;
}

/** the body of POST /me/decks/approve (null when the code can't be one) */
export function approveBody(
	code: string,
	days: number,
	idleDays: number,
	name: string
): ApproveBody | null {
	const normalized = normalizeCode(code);
	if (!normalized) return null;
	const body: ApproveBody = { code: normalized, days };
	if (idleDays > 0) body.idle_days = idleDays;
	const trimmed = name.trim().replace(/\s+/g, ' ');
	if (trimmed) body.name = trimmed.slice(0, 100);
	return body;
}

/** the pairing link's code (`?deck=`), formatted for the field; '' when there is none */
export function codeFromUrl(url: URL): string {
	const code = normalizeCode(url.searchParams.get('deck'));
	return code ? formatCode(code) : '';
}
