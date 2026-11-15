// #518 (plan E2b, agents/research/SCENE-EVENTS.md §4; LIVE.md B.1): where the story events the reader fires go. Normal
// reading: nowhere – no session, nothing is held or sent. A reader joined to a session (reader-session.ts, loaded only
// then) listens here and tells the session each event as it fires, the VNEvent as it is. Which events matter is the
// receivers' (deck-events adapters/story.rs `STAYS_IN_THE_STORY`): no list of types, no filter here.
// #552: the way back – a story question's vote result the core says is this reader's choice is picked here, by the
// choice buttons (choices.svelte), as a tap would; which option, and whether at all, is the core's (deck-space linked.rs).
import type { VNEvent } from '@scutoons/content';

/** what a session hears of the reader's story */
export interface SessionEar {
	/** an event passed its condition and runs */
	fired(event: VNEvent): void;
	/** #552: a `showChoices` is over (picked, or nothing to ask) */
	closed(): void;
}

let session: SessionEar | null = null;
let picker: ((index: number) => boolean) | null = null;

/** an event passed its condition and runs (the runner's `fired`): the session hears it, if there is one */
export function tellSession(event: VNEvent): void {
	session?.fired(event);
}

/** #552: the runner's `choicesClosed`: the session hears it, if there is one */
export function tellChoicesClosed(): void {
	session?.closed();
}

/** the session's ear (reader-session.ts); null: none again */
export function listenSession(ear: SessionEar | null): void {
	session = ear;
}

/** #552: the choice buttons on the page (choices.svelte) pick the open choice at `index` as a tap would; the newest
 *  listens. Returns its release (it stops listening, unless a newer one took over). */
export function listenPick(fn: (index: number) => boolean): () => void {
	picker = fn;
	return () => void (picker === fn && (picker = null));
}

/** #552: pick the open choice at `index` (its place among the options shown); false: no choice buttons, or none there */
export function pickChoice(index: number): boolean {
	return picker?.(index) ?? false;
}
