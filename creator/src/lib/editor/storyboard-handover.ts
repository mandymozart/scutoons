// A shot hands over by a link (research/redesign/lap2/STORYBOARD.md "Where a shot goes"): to its slice in the chapter
// (the chapter workspace finds it by `Slice.shot`, or cuts it on first use – chapter-shot.ts), to the deck (its scene, or
// the shot's own address so the deck makes one, #873) and to its XR experience. The addresses are the core's
// (@scutoons/content/storyboard handover.ts); where an address opens in the deck is the core's too (the caller passes the
// workspace). Pure: test/storyboard-handover.test.ts.
import { deckLink } from '@scutoons/api';
import { fitsMade, shotAddress, type ShotMade } from '@scutoons/content/storyboard';
import { experienceHref } from './experiences.ts';

/** the query parameter that opens a chapter on the slice cut from a shot */
export const SHOT_PARAM = 'shot';

/** the chapter workspace on the slice cut from `shot` (made there on first use) */
export const sliceHref = (toon: string, chapter: number, shot: string): string => `/${toon}/chapter/${chapter}?${SHOT_PARAM}=${encodeURIComponent(shot)}`;

/** what the deck link opens: the shot's deck scene, else the shot itself – the deck makes its scene from it */
export function deckTarget(made: ShotMade | undefined, toon: string, chapter: number, shot: string): { address: string; made: boolean } {
	return fitsMade('deck', made?.deck) ? { address: made.deck, made: true } : { address: shotAddress(toon, chapter, shot), made: false };
}

/** the deck link: `workspace` is where the core opens the target (a made scene: its own answer; the shot: a scene's) */
export const shotDeckLink = (workspace: string | null | undefined, address: string): string | null => (workspace ? deckLink(workspace, address) : null);

/** the XR experience the shot was made into; none: null (the creator cannot make one – an experience is imported) */
export function xrHref(made: ShotMade | undefined): string | null {
	return fitsMade('xr', made?.xr) ? experienceHref(made.xr.slice('experience:'.length)) : null;
}
