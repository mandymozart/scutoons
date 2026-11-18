// The creators' docs (/docs, public): the use cases grouped by phase, one tutorial each. The data is tutorials.ts.

export type Who = 'all' | 'writer' | 'artist' | 'sound' | 'performer' | 'lead';
/** creator = creator.scutoons.com · editor = the deck editor · deck = the native deck */
export type Where = 'creator' | 'editor' | 'deck';
/** 3 = every session · 2 = weekly / per scene · 1 = per project / once */
export type Often = 1 | 2 | 3;

export interface Step {
	text: string;
	/** a short name of the step for lists; falls back to the text */
	label?: string;
}

export interface Fact {
	label: string;
	text: string;
}

export interface Case {
	slug: string;
	title: string;
	summary: string;
	who: Who[];
	where: Where[];
	often: Often;
	/** built only in part: the tutorial says what works today */
	partly?: boolean;
	facts?: Fact[];
	/** no steps yet = listed in the index, its tutorial is still to be written */
	steps?: Step[];
	note?: string;
}

export interface Group {
	key: string;
	title: string;
	intro: string;
	cases: Case[];
}

export const WHO: Record<Who, string> = {
	all: 'Every creator',
	writer: 'Writer',
	artist: 'Artist',
	sound: 'Sound designer',
	performer: 'Performer',
	lead: 'Lead'
};

export const WHERE: Record<Where, string> = {
	creator: 'Creator',
	editor: 'Deck editor',
	deck: 'Deck'
};

export const OFTEN: Record<Often, string> = {
	3: 'Every session',
	2: 'Often',
	1: 'Now and then'
};

export function allCases(groups: Group[]): Case[] {
	return groups.flatMap((g) => g.cases);
}

export function findCase(groups: Group[], slug: string): { group: Group; item: Case } | undefined {
	for (const group of groups) {
		const item = group.cases.find((c) => c.slug === slug);
		if (item) return { group, item };
	}
	return undefined;
}

/** a tutorial is written when it has its steps */
export function written(item: Case): boolean {
	return (item.steps?.length ?? 0) > 0;
}

/** the next written tutorial after this one, across groups; none after the last */
export function nextCase(groups: Group[], slug: string): Case | undefined {
	const list = allCases(groups);
	const at = list.findIndex((c) => c.slug === slug);
	return at < 0 ? undefined : list.slice(at + 1).find(written);
}
