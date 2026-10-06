// Sample readers (agents/EXPRESSIONS.md §4 + step 4): fixed answers for the creator's previews of texts with `{…}` – the
// builder's live preview and the slice canvas. Three readers ("new guest", "halfway", "everything collected") answer
// every registry name (EXPR_NAMESPACES / EXPR_FUNCTIONS) the way such a reader would; story variables are unknown to
// the creator in kind, so they are unset / 3 / 10. `now.*` is the creator's own clock. Pure (tests: expr-edit.test.ts).
import { isStoryPath, type ExprResolver, type ExprValue } from '@scutoons/content';

export const SAMPLE_READERS = ['guest', 'halfway', 'complete'] as const;
export type SampleReaderId = (typeof SAMPLE_READERS)[number];

interface Fixture {
	variable: number | undefined;
	reader: Record<string, ExprValue>;
	chapters: number;
	finished: boolean;
	position: number;
	has: boolean;
	count: number;
	items: number;
	edition: number | undefined;
	avatar: { character: string; look: string; wears: boolean; unlocked: number };
	stats: Record<string, number>;
	effects: { active: boolean; count: number };
	map: { unlocked: number; seen: number; total: number; place: boolean; here: string };
	resolved: boolean;
	events: number;
	/** how often each list was shown before: the first item / the second / the last */
	visits: number;
}

const FIXTURES: Record<SampleReaderId, Fixture> = {
	guest: {
		variable: undefined,
		reader: { name: undefined, signedIn: false, member: false, memberDays: 0, since: undefined, gifts: 0, daysSinceStart: 0 },
		chapters: 0,
		finished: false,
		position: 0,
		has: false,
		count: 0,
		items: 0,
		edition: undefined,
		avatar: { character: 'boy', look: 'default', wears: false, unlocked: 1 },
		stats: { testo: 0, appetite: 50 },
		effects: { active: false, count: 0 },
		map: { unlocked: 1, seen: 1, total: 12, place: false, here: 'Home' },
		resolved: false,
		events: 0,
		visits: 0
	},
	halfway: {
		variable: 3,
		reader: { name: 'Mia', signedIn: true, member: false, memberDays: 0, since: undefined, gifts: 1, daysSinceStart: 12 },
		chapters: 3,
		finished: false,
		position: 0.5,
		has: true,
		count: 1,
		items: 4,
		edition: 12,
		avatar: { character: 'boy', look: 'default', wears: false, unlocked: 2 },
		stats: { testo: 40, appetite: 50 },
		effects: { active: false, count: 1 },
		map: { unlocked: 6, seen: 8, total: 12, place: true, here: 'Market' },
		resolved: true,
		events: 1,
		visits: 1
	},
	complete: {
		variable: 10,
		reader: { name: 'Mia', signedIn: true, member: true, memberDays: 400, since: '2025-08-01', gifts: 5, daysSinceStart: 300 },
		chapters: 10,
		finished: true,
		position: 1,
		has: true,
		count: 5,
		items: 20,
		edition: 1,
		avatar: { character: 'catpope', look: 'gala', wears: true, unlocked: 5 },
		stats: { testo: 100, appetite: 80 },
		effects: { active: true, count: 3 },
		map: { unlocked: 12, seen: 12, total: 12, place: true, here: 'Cathedral' },
		resolved: true,
		events: 3,
		visits: 99
	}
};

const pad = (n: number) => String(n).padStart(2, '0');

/**
 * The resolver of a sample reader for one text in `language`: story variables (bare names) per the fixture, the
 * registry's namespaces, the state functions (`has`, `count_of`, `read_chapter`, `resolved`, `count`, `inventory.has` …).
 */
export function sampleResolver(id: SampleReaderId, language: string, now: Date = new Date()): ExprResolver {
	const f = FIXTURES[id] ?? FIXTURES.guest;
	const names: Record<string, Record<string, ExprValue>> = {
		reader: { ...f.reader, language },
		read: { chapters: f.chapters, finished: f.finished },
		inventory: { items: f.items },
		avatar: { character: f.avatar.character, look: f.avatar.look, unlocked: f.avatar.unlocked },
		stats: f.stats,
		effects: { count: f.effects.count },
		map: { unlocked: f.map.unlocked, seen: f.map.seen, total: f.map.total, here: f.map.here },
		now: {
			hour: now.getHours(),
			weekday: ((now.getDay() + 6) % 7) + 1,
			night: now.getHours() >= 22 || now.getHours() < 6,
			date: `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`
		}
	};
	const calls: Record<string, (args: readonly ExprValue[]) => ExprValue> = {
		has: () => f.has,
		count_of: () => f.count,
		read_chapter: ([n]) => typeof n === 'number' && n <= f.chapters,
		resolved: () => f.resolved,
		count: () => f.events,
		'read.chapter': ([n]) => typeof n === 'number' && n <= f.chapters,
		'read.position': () => f.position,
		'inventory.has': () => f.has,
		'inventory.count': () => f.count,
		'inventory.edition': () => f.edition,
		'avatar.wears': () => f.avatar.wears,
		'effects.active': () => f.effects.active,
		'map.place': () => f.map.place
	};
	return {
		// a story variable as written (`coins`, `global.coins`, `toptoy.coins`) or a namespace member
		// (`inventory.<item>` = the fixture's count, like `inventory.count("<item>")`)
		name: (path) => (isStoryPath(path) ? f.variable : (names[path[0]]?.[path[1]] ?? (path[0] === 'inventory' && path.length === 2 ? f.count : undefined))),
		call: (name, args) => (Object.hasOwn(calls, name) ? calls[name](args) : undefined),
		visit: () => f.visits,
		seed: id,
		language
	};
}
