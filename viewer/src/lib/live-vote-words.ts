// The audience phone's words in the device's language (owner 2026-11-01: "language of the device"). The question and the
// option labels come with the vote (`question`, `labels` – locale maps), these are the rest. de and en; any other
// language shows English.
import type { HowKey } from './live-vote.ts';

export interface VoteWords {
	vote: string;
	left: (s: number) => string;
	paused: string;
	pick: string;
	change: string;
	waiting: string;
	result: string;
	yours: string;
	closed: string;
	how: Record<HowKey, string>;
}

export const WORDS: Record<'en' | 'de', VoteWords> = {
	en: {
		vote: 'Vote',
		left: (s) => `${s} s left`,
		paused: 'on hold',
		pick: 'Tap your choice.',
		change: 'You can change it until the end.',
		waiting: 'Waiting for the next vote.',
		result: 'Result',
		yours: 'Your vote',
		closed: 'The vote has closed.',
		how: { majority: 'Most votes.', groups: 'Most groups.', first: 'First to enough votes.', fallback: 'Nobody voted – it stays as it was.', override: 'The conductor decided.' }
	},
	de: {
		vote: 'Abstimmung',
		left: (s) => `noch ${s} s`,
		paused: 'angehalten',
		pick: 'Tipp auf deine Wahl.',
		change: 'Du kannst bis zum Schluss wechseln.',
		waiting: 'Warte auf die nächste Abstimmung.',
		result: 'Ergebnis',
		yours: 'Deine Stimme',
		closed: 'Die Abstimmung ist beendet.',
		how: { majority: 'Die meisten Stimmen.', groups: 'Die meisten Gruppen.', first: 'Zuerst genug Stimmen.', fallback: 'Niemand hat abgestimmt – es bleibt dabei.', override: 'Die Regie hat entschieden.' }
	}
};

export const wordsFor = (locale: string): VoteWords => (locale === 'de' ? WORDS.de : WORDS.en);
