// The live vote's shared bits (#231): what a vote screen talks to, and the words a phone shows in its device's language.
import type { Envelope, VoteOpen, VoteResult } from '@scutoons/live';

/** What the vote screens listen and talk to – the real relay (live-vote-link.ts) or the draft's fake (live-vote-sim.ts). */
export interface VoteSource {
	/** listen as the conductor (everything) or as an audience phone; an open vote reaches a late listener at once */
	on(role: 'conductor' | 'audience', fn: (e: Envelope) => void, group?: string, client?: string): () => void;
	/** an audience phone's vote.cast (changeable until the close) */
	cast(client: string, option: number, group?: string): void;
	/** conductor: vote.open */
	open(p: VoteOpen): void;
	/** conductor: vote.close – Close now, or Override with an option */
	close(override?: number): void;
	/** conductor: vote.hold – stop the countdown (votes still count) / run it on */
	hold(hold: boolean): void;
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

/** `vote.result` → what to say about how it was decided */
export type HowKey = 'majority' | 'groups' | 'first' | 'fallback' | 'override';
export const howKey = (open: Pick<VoteOpen, 'rule'>, result: Pick<VoteResult, 'how'>): HowKey => (result.how === 'rule' ? open.rule : result.how);

/** the share of `c` in the counts, in whole percent */
export const percent = (c: number, counts: number[]): number => {
	const total = counts.reduce((a, b) => a + b, 0);
	return total ? Math.round((c / total) * 100) : 0;
};

/** What the conductor types to open a vote (until the show's choice events open them: one line per option). */
export interface VoteForm {
	id: string;
	/** the question by locale (empty = not given) */
	question: { en: string; de: string };
	/** one option per line, by locale; `en` is the default `options`, `de` is optional but needs the same number of lines */
	options: { en: string; de: string };
	rule: VoteOpen['rule'];
	n: number;
	seconds: number;
	fallback: number;
	/** group ids separated by commas; empty = the whole audience */
	groups: string;
	show: boolean;
}

const lines = (s: string): string[] => s.split('\n').map((l) => l.trim()).filter(Boolean);

/** The form → `vote.open`, or what is wrong with it. */
export function buildOpen(f: VoteForm): { open: VoteOpen } | { error: string } {
	const options = lines(f.options.en);
	if (options.length < 2) return { error: 'A vote needs at least two options (one per line).' };
	if (options.length > 6) return { error: 'At most six options.' };
	const de = lines(f.options.de);
	if (de.length && de.length !== options.length) return { error: 'The German options need the same number of lines.' };
	if (!(f.seconds >= 1)) return { error: 'Seconds: at least 1.' };
	if (f.rule === 'first' && !(f.n >= 1)) return { error: 'First to n: n is at least 1.' };
	if (f.fallback < 0 || f.fallback >= options.length) return { error: 'The fallback is one of the options.' };
	const question = Object.fromEntries(Object.entries(f.question).map(([k, v]) => [k, v.trim()]).filter(([, v]) => v));
	const groups = f.groups.split(',').map((g) => g.trim()).filter(Boolean);
	return {
		open: {
			vote: f.id,
			options,
			rule: f.rule,
			...(f.rule === 'first' ? { n: Math.floor(f.n) } : {}),
			seconds: Math.floor(f.seconds),
			fallback: f.fallback,
			...(groups.length ? { groups } : {}),
			show: f.show,
			...(Object.keys(question).length ? { question } : {}),
			...(de.length ? { labels: { en: options, de } } : {})
		}
	};
}

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
