// deck-key.ts (#470): the browser deck's identity and the I/O of its pairing. The key is an Ed25519 pair WebCrypto makes
// NON-EXTRACTABLE and IndexedDB keeps (a script on the page can sign with it, never read it); the handshake's decisions and
// its schedule are deck-link's (space-web `DeckLink`) – this only fetches what an ask names and signs what it names.
// No WebCrypto Ed25519 (an old browser), no IndexedDB (a private window): no pairing – public media still play.

/** the wasm-bindgen API of deck/space-web's `DeckLink` */
export interface DeckLinkView {
	next(now: number): string | undefined;
	replied(status: number, body: string, now: number): string | undefined;
	signed(signatureHex: string, now: number): string | undefined;
	failed(why: string, now: number): void;
	forget(now: number): void;
	token(now: number): string | undefined;
	code(): string;
	why(now: number): string;
	line(now: number): string;
	free(): void;
}

/** the page's link: the core's view, the key it signs with, a round on its way */
export interface Paired {
	view: DeckLinkView;
	key: CryptoKey;
	busy: boolean;
}

const STORE = 'keys';

const done = <T>(r: IDBRequest<T>) => new Promise<T>((ok, no) => ((r.onsuccess = () => ok(r.result)), (r.onerror = () => no(r.error))));

async function keys(mode: IDBTransactionMode): Promise<IDBObjectStore> {
	const open = indexedDB.open('scutoons-deck', 1);
	open.onupgradeneeded = () => open.result.createObjectStore(STORE);
	return (await done(open)).transaction(STORE, mode).objectStore(STORE);
}

const hex = (b: ArrayBuffer) => Array.from(new Uint8Array(b), (x) => x.toString(16).padStart(2, '0')).join('');

/** The deck's key pair (made once, non-extractable) and its public key as hex; null = this browser cannot pair */
export async function deckKey(): Promise<{ pair: CryptoKeyPair; publicHex: string } | null> {
	try {
		let pair = (await done((await keys('readonly')).get('deck'))) as CryptoKeyPair | undefined;
		if (!pair) {
			pair = (await crypto.subtle.generateKey({ name: 'Ed25519' }, false, ['sign', 'verify'])) as CryptoKeyPair;
			await done((await keys('readwrite')).put(pair, 'deck'));
		}
		return { pair, publicHex: hex(await crypto.subtle.exportKey('raw', pair.publicKey)) };
	} catch (e) {
		console.warn('deck: this browser cannot pair (WebCrypto Ed25519 / IndexedDB)', e);
		return null;
	}
}

/** One handshake round when it is due (one at a time): POST what `post` names under `api` (no token, no cookies), sign
 *  what `sign` names */
export async function driveLink(link: Paired, api: string): Promise<void> {
	if (link.busy) return;
	link.busy = true;
	const { view } = link;
	let ask = view.next(performance.now());
	try {
		while (ask) {
			const a = JSON.parse(ask) as { post?: string; body?: unknown; sign?: string };
			if (a.sign !== undefined) {
				ask = view.signed(hex(await crypto.subtle.sign('Ed25519', link.key, new TextEncoder().encode(a.sign))), performance.now());
			} else {
				const r = await fetch(api + a.post, { method: 'POST', credentials: 'omit', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(a.body) });
				ask = view.replied(r.status, await r.text(), performance.now());
			}
		}
	} catch (e) {
		view.failed((e as Error).message || 'offline', performance.now());
	} finally {
		link.busy = false;
	}
}

/** the outlet's side of the API's answers (space-web SceneView): a signed link, the deck's proof (#478) */
export interface SignedView {
	outlet_signed(key: string, status: number, body: string): void;
	outlet_proof(status: number, body: string): void;
}

/** #478: the deck's proof (the outlet's `r:` ask) – the reader's sign-on cookie goes to the API's proof route and nowhere else */
export async function fetchProof(host: { view: SignedView; stopped: boolean }, o: { api: string }, url: string) {
	if (!url.startsWith(`${o.api}/live/proof?`)) throw new Error('not the API');
	const r = await fetch(url, { credentials: 'include', cache: 'no-store' });
	if (!host.stopped) host.view.outlet_proof(r.status, await r.text());
}

/** #470: a show medium's signed link (the outlet's `s:` ask) – the deck token goes to the API's show-media route and nowhere else */
export async function fetchShowLink(host: { view: SignedView; stopped: boolean }, o: { api: string; link?: DeckLinkView }, key: string, url: string) {
	const token = o.link?.token(performance.now());
	if (!url.startsWith(`${o.api}/decks/media?`)) throw new Error('not the API');
	if (!token) throw new Error('the deck has no token');
	const r = await fetch(url, { credentials: 'omit', headers: { Authorization: `Bearer ${token}` } });
	if (r.status === 401) o.link?.forget(performance.now());
	if (!host.stopped) host.view.outlet_signed(key, r.status, await r.text());
}
