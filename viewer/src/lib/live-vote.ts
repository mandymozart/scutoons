// The live vote's shared bits (#231): what the audience's vote screen talks to, and the words a phone shows in its
// device's language. The conductor's side is the deck editor's Perform workspace (#533, deck-show vote.rs).
import type { Envelope, VoteOpen, VoteResult } from '@scutoons/live';

/** What the audience's vote screen listens and talks to – the real relay (live-vote-link.ts) or the lab's fake
 *  (live-vote-sim.ts). */
export interface VoteSource {
	/** listen as an audience phone; an open vote reaches a late listener at once */
	on(role: 'audience', fn: (e: Envelope) => void, group?: string, client?: string): () => void;
	/** an audience phone's vote.cast (changeable until the close) */
	cast(client: string, option: number, group?: string): void;
}

/** the device's language, two letters, lower case (`de-AT` → `de`) */
export const deviceLocale = (languages: readonly string[] | undefined): string => (languages?.[0] ?? 'en').slice(0, 2).toLowerCase();

/** A locale map's text for the device: its language, else English, else the first one, else `fallback`. */
export function pickText(map: Record<string, string> | undefined, locale: string, fallback = ''): string {
	if (!map) return fallback;
	return map[locale] ?? map.en ?? Object.values(map)[0] ?? fallback;
}

/** The option labels for the device: the vote's `labels` in its language, else English, else `options`. */
export function pickLabels(open: Pick<VoteOpen, 'options' | 'labels'>, locale: string): string[] {
	const l = open.labels?.[locale] ?? open.labels?.en;
	return l && l.length === open.options.length ? l : open.options;
}

/** `vote.result` → what to say about how it was decided; a `how` this page has no words for (`tie`, #558: its words
 *  come with #557) is worded as the rule */
export type HowKey = 'majority' | 'groups' | 'first' | 'fallback' | 'override';
export const howKey = (open: Pick<VoteOpen, 'rule'>, result: Pick<VoteResult, 'how'>): HowKey =>
	result.how === 'fallback' || result.how === 'override' ? result.how : open.rule;

/** the share of `c` in the counts, in whole percent */
export const percent = (c: number, counts: number[]): number => {
	const total = counts.reduce((a, b) => a + b, 0);
	return total ? Math.round((c / total) * 100) : 0;
};

/** A group's colour on the phone (the show's group colours, B.5.3, are not served yet): one of the badge colours by the
 *  group's id, and the text colour that reads on it. */
const LOOKS = [
	{ colour: 'var(--badge-published)', text: 'var(--badge-published-text)' },
	{ colour: 'var(--badge-warning)', text: 'var(--badge-warning-text)' },
	{ colour: 'var(--badge-soon)', text: 'var(--badge-soon-text)' },
	{ colour: 'var(--badge-info)', text: 'var(--badge-info-text)' },
	{ colour: 'var(--badge-error)', text: 'var(--badge-error-text)' }
] as const;
export function groupLook(id: string): { colour: string; text: string } {
	let h = 0;
	for (const ch of id) h = (h * 31 + ch.charCodeAt(0)) >>> 0;
	return LOOKS[h % LOOKS.length];
}
