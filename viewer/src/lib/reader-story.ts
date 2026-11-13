// #518 (plan E2b, agents/research/SCENE-EVENTS.md §4; LIVE.md B.1): where the story events the reader fires go. Normal
// reading: nowhere – no session, nothing is held or sent. A reader joined to a session (reader-session.ts, loaded only
// then) listens here and tells the session each event as it fires, the VNEvent as it is. Which events matter is the
// receivers' (deck-events adapters/story.rs `STAYS_IN_THE_STORY`): no list of types, no filter here.
import type { VNEvent } from '@scutoons/content';

let session: ((event: VNEvent) => void) | null = null;

/** an event passed its condition and runs (the runner's `fired`): the session hears it, if there is one */
export function tellSession(event: VNEvent): void {
	session?.(event);
}

/** the session's ear (reader-session.ts); null: none again */
export function listenSession(fn: ((event: VNEvent) => void) | null): void {
	session = fn;
}
