// phone-proof.ts (#478, research/LIVE-CAMERAS.md §4): a reader phone's proof for its live session. The core asks for it
// (`{ ask_proof: { url } }` – when, and again at half its life) and reads the answer; this only fetches it with the
// person's sign-in (deck-key.ts `fetchProof`: the API's proof route and nowhere else) and hands the bytes back.
import { fetchProof, type SignedIn } from './deck-key.ts';
import type { HandLink } from './hand.ts';

interface ProofCore {
	proof(now: number, status: number, body: string): void;
}

/** the API (the proof's route) and who is signed in (none: the phone is a guest) */
export interface PhoneProof {
	api?: string;
	user?: SignedIn;
}

/** the proof the core asked for at `url`, to the core; no answer is an answer too (the core asks again later) */
export async function phoneProof(link: HandLink, o: PhoneProof, url: string): Promise<void> {
	let answer: [number, string] = [0, ''];
	const view = { outlet_proof: (status: number, body: string) => void (answer = [status, body]) };
	await fetchProof({ view, stopped: false }, { api: o.api ?? '', user: o.user }, url).catch(() => undefined);
	(link.s?.core as unknown as ProofCore | undefined)?.proof(Date.now(), ...answer);
	link.s?.run();
}
