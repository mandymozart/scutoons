// Expressions in texts (agents/EXPRESSIONS.md §1–2): THE registry – every function a text can call and every name a
// reader resolver answers. Functions are defined here only (owner 2026-10-06: no author-defined functions); the builder
// lists them, the analyser, the evaluator and the server check (server/src/Domain/Content/Expr/Registry.php, kept equal
// by the golden fixture test/fixtures/expr/registry.json) read them. Adding a function = an entry in EXPR_FUNCTIONS and,
// when it is pure (doesn't read the reader's state), its implementation in EXPR_IMPLS; a state function is answered by
// the reader's resolver (`call(name, args)`).
import type { ExprValue } from './ast.ts';

export type ExprType = 'number' | 'string' | 'boolean' | 'any';

export interface ExprParam {
	name: string;
	type: ExprType;
}

export interface ExprText {
	de: string;
	en: string;
}

export interface ExprFunction {
	name: string;
	params: ExprParam[];
	/** the last parameter repeats (`min(a, b, c)`) */
	variadic?: boolean;
	result: ExprType;
	/** reads the reader's state: answered by the resolver's `call`, not by EXPR_IMPLS */
	state?: boolean;
	description: ExprText;
	example: string;
}

export interface ExprMember {
	/** the member's name inside its namespace (`total` of `map.total`) */
	name: string;
	/** a value (`map.total`) or a function (`inventory.has("x")`) */
	kind: 'value' | 'function';
	type: ExprType;
	params?: ExprParam[];
	description: ExprText;
}

export interface ExprNamespace {
	name: string;
	description: ExprText;
	members: ExprMember[];
}

const n = (name: string): ExprParam => ({ name, type: 'number' });
const s = (name: string): ExprParam => ({ name, type: 'string' });
const a = (name: string): ExprParam => ({ name, type: 'any' });

export const EXPR_FUNCTIONS: readonly ExprFunction[] = [
	{ name: 'clock', params: [n('minutes')], result: 'string', description: { de: 'Minuten seit Mitternacht als Uhrzeit', en: 'Minutes since midnight as a time of day' }, example: '{clock(330)} → 5:30' },
	{ name: 'upper', params: [a('text')], result: 'string', description: { de: 'In Großbuchstaben', en: 'In capital letters' }, example: '{upper("hey")} → HEY' },
	{ name: 'lower', params: [a('text')], result: 'string', description: { de: 'In Kleinbuchstaben', en: 'In small letters' }, example: '{lower("HEY")} → hey' },
	{ name: 'capitalize', params: [a('text')], result: 'string', description: { de: 'Erster Buchstabe groß', en: 'First letter capital' }, example: '{capitalize(reader.name)} → Mia' },
	{ name: 'number', params: [n('value')], result: 'string', description: { de: 'Zahl im Format der Textsprache', en: 'A number in the text language’s format' }, example: '{number(1234.5)} → 1,234.5 (en) / 1.234,5 (de)' },
	{ name: 'min', params: [n('a'), n('b')], variadic: true, result: 'number', description: { de: 'Die kleinste Zahl', en: 'The smallest number' }, example: '{min(coins, 10)}' },
	{ name: 'max', params: [n('a'), n('b')], variadic: true, result: 'number', description: { de: 'Die größte Zahl', en: 'The largest number' }, example: '{max(0, coins - 5)}' },
	{ name: 'round', params: [n('value')], result: 'number', description: { de: 'Gerundet auf eine ganze Zahl', en: 'Rounded to a whole number' }, example: '{round(2.5)} → 3' },
	{ name: 'floor', params: [n('value')], result: 'number', description: { de: 'Abgerundet auf eine ganze Zahl', en: 'Rounded down to a whole number' }, example: '{floor(minutes / 60)}' },
	{ name: 'random', params: [n('min'), n('max')], result: 'number', description: { de: 'Eine ganze Zufallszahl von min bis max – je Leser und Text fest', en: 'A whole random number from min to max – fixed per reader and text' }, example: '{random(1, 6)}' },
	{ name: 'has', params: [s('item')], result: 'boolean', state: true, description: { de: 'Hat der Leser das Item?', en: 'Does the reader have the item?' }, example: '{has("cucumber"): You smell of cucumber.}' },
	{ name: 'count_of', params: [s('item')], result: 'number', state: true, description: { de: 'Wie viele des Items der Leser hat', en: 'How many of the item the reader has' }, example: '{count_of("coin")} coins' },
	{ name: 'read_chapter', params: [n('chapter')], result: 'boolean', state: true, description: { de: 'Hat der Leser das Kapitel gelesen?', en: 'Has the reader read the chapter?' }, example: '{read_chapter(2): Remember the lake?}' },
	{ name: 'resolved', params: [s('event')], result: 'boolean', state: true, description: { de: 'Ist das Ereignis schon passiert?', en: 'Has the event happened yet?' }, example: '{resolved(event): Welcome back.}' },
	{ name: 'count', params: [s('event')], result: 'number', state: true, description: { de: 'Wie oft das Ereignis passiert ist', en: 'How often the event has happened' }, example: '{count(event)} times' },
];

const v = (name: string, type: ExprType, de: string, en: string): ExprMember => ({ name, kind: 'value', type, description: { de, en } });
const f = (name: string, params: ExprParam[], type: ExprType, de: string, en: string): ExprMember => ({ name, kind: 'function', type, params, description: { de, en } });

/** the reader state a text can read (EXPRESSIONS.md §2) – never email, codes, account ids or anything about other readers */
export const EXPR_NAMESPACES: readonly ExprNamespace[] = [
	{
		name: 'reader',
		description: { de: 'Der Leser', en: 'The reader' },
		members: [
			v('name', 'string', 'Name des Lesers (askText, SCUTOONS-weit)', 'The reader’s name (askText, SCUTOONS-wide)'),
			v('signedIn', 'boolean', 'Angemeldet?', 'Signed in?'),
			v('member', 'boolean', 'Mitglied?', 'A member?'),
			v('memberDays', 'number', 'Tage als Mitglied', 'Days as a member'),
			v('since', 'string', 'Mitglied seit (Datum)', 'Member since (date)'),
			v('gifts', 'number', 'Erhaltene Geschenke', 'Gifts received'),
			v('language', 'string', 'Lesesprache', 'Reading language'),
			v('daysSinceStart', 'number', 'Tage seit dem ersten Lesen', 'Days since first reading'),
		],
	},
	{
		name: 'read',
		description: { de: 'Lesefortschritt', en: 'Reading progress' },
		members: [
			f('chapter', [n('chapter')], 'boolean', 'Kapitel gelesen?', 'Chapter read?'),
			v('chapters', 'number', 'Gelesene Kapitel', 'Chapters read'),
			v('finished', 'boolean', 'Toon zu Ende gelesen?', 'Toon read to the end?'),
			f('position', [n('chapter')], 'number', 'Wie weit im Kapitel (Slice)', 'How far into the chapter (slice)'),
		],
	},
	{
		name: 'inventory',
		description: { de: 'Inventar', en: 'Inventory' },
		members: [
			f('has', [s('item')], 'boolean', 'Hat das Item?', 'Has the item?'),
			f('count', [s('item')], 'number', 'Anzahl des Items', 'Count of the item'),
			v('items', 'number', 'Anzahl verschiedener Items', 'Number of different items'),
			f('edition', [s('item')], 'number', 'Editionsnummer des Items', 'The item’s edition number'),
		],
	},
	{
		name: 'avatar',
		description: { de: 'Avatar', en: 'Avatar' },
		members: [
			v('character', 'string', 'Gewählte Figur', 'Chosen character'),
			v('look', 'string', 'Aktueller Look', 'Current look'),
			f('wears', [s('item')], 'boolean', 'Trägt das Item?', 'Wears the item?'),
			v('unlocked', 'number', 'Freigeschaltete Figuren', 'Unlocked characters'),
		],
	},
	{
		name: 'stats',
		description: { de: 'Werte', en: 'Stats' },
		members: [v('testo', 'number', 'Testo', 'Testo'), v('appetite', 'number', 'Appetit', 'Appetite')],
	},
	{
		name: 'effects',
		description: { de: 'Wirkungen', en: 'Effects' },
		members: [f('active', [s('effect')], 'boolean', 'Wirkung aktiv?', 'Effect active?'), v('count', 'number', 'Aktive Wirkungen', 'Active effects')],
	},
	{
		name: 'map',
		description: { de: 'Karte', en: 'Map' },
		members: [
			v('unlocked', 'number', 'Freigeschaltete Orte', 'Unlocked places'),
			v('seen', 'number', 'Gesehene Orte', 'Places seen'),
			v('total', 'number', 'Orte insgesamt', 'Places in total'),
			f('place', [s('place')], 'boolean', 'Ort freigeschaltet?', 'Place unlocked?'),
			v('here', 'string', 'Aktueller Ort', 'Current place'),
		],
	},
	{
		name: 'now',
		description: { de: 'Jetzt (Leserzeit)', en: 'Now (reader’s time)' },
		members: [
			v('hour', 'number', 'Stunde 0–23', 'Hour 0–23'),
			v('weekday', 'number', 'Wochentag 1 (Mo) – 7 (So)', 'Weekday 1 (Mon) – 7 (Sun)'),
			v('night', 'boolean', 'Nacht?', 'Night?'),
			v('date', 'string', 'Datum JJJJ-MM-TT', 'Date YYYY-MM-DD'),
		],
	},
];

/** minutes since midnight as a clock: 330 → "5:30", 1445 → "0:05" (as vn-script formatClock) */
const clock = (minutes: number): string => {
	const m = ((Math.round(minutes) % 1440) + 1440) % 1440;
	return `${Math.floor(m / 60)}:${String(m % 60).padStart(2, '0')}`;
};

export interface ExprImplContext {
	/** the text's language (BCP 47) */
	language: string;
	/** a number in [0, 1) – fixed per resolver seed and call site */
	random(): number;
	/** a value as text (ink's printing) */
	show(value: ExprValue): string;
}

/** the pure functions' implementations: undefined = can't use these arguments */
export type ExprImpl = (args: ExprValue[], ctx: ExprImplContext) => ExprValue;

const num = (x: ExprValue): number | undefined => (typeof x === 'number' ? x : typeof x === 'boolean' ? Number(x) : undefined);
const nums = (args: ExprValue[]): number[] | undefined => {
	const all = args.map(num);
	return all.length && all.every((x) => x !== undefined) ? (all as number[]) : undefined;
};
const locale = (lang: string, fn: () => string, fallback: () => string): string => {
	try {
		return fn();
	} catch {
		return fallback();
	}
};

export const EXPR_IMPLS: Readonly<Record<string, ExprImpl>> = {
	clock: ([t]) => (num(t) === undefined ? undefined : clock(num(t)!)),
	upper: ([x], c) => locale(c.language, () => c.show(x).toLocaleUpperCase(c.language), () => c.show(x).toUpperCase()),
	lower: ([x], c) => locale(c.language, () => c.show(x).toLocaleLowerCase(c.language), () => c.show(x).toLowerCase()),
	capitalize: ([x], c) => {
		const t = c.show(x);
		const first = String.fromCodePoint(t.codePointAt(0) ?? 32).trim();
		return locale(c.language, () => first.toLocaleUpperCase(c.language), () => first.toUpperCase()) + t.slice(first.length);
	},
	number: ([x], c) => {
		const value = num(x);
		return value === undefined ? undefined : locale(c.language, () => new Intl.NumberFormat(c.language).format(value), () => String(value));
	},
	min: (args) => {
		const all = nums(args);
		return all && Math.min(...all);
	},
	max: (args) => {
		const all = nums(args);
		return all && Math.max(...all);
	},
	round: ([x]) => (num(x) === undefined ? undefined : Math.round(num(x)!)),
	floor: ([x]) => (num(x) === undefined ? undefined : Math.floor(num(x)!)),
	random: (args, c) => {
		const all = nums(args);
		if (!all) return undefined;
		const lo = Math.ceil(Math.min(all[0], all[1]));
		const hi = Math.floor(Math.max(all[0], all[1]));
		return hi < lo ? undefined : lo + Math.floor(c.random() * (hi - lo + 1));
	},
};
