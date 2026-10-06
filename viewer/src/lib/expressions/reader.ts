// The reader's answers for expressions in texts (agents/EXPRESSIONS.md §2, ink's inline syntax): every name and state
// function of the registry's namespaces (EXPR_NAMESPACES / EXPR_FUNCTIONS in @scutoons/content) read from the viewer's
// state – pure, over getters (resolver.svelte.ts wires them to the contexts; the getters read reactive state, so a text
// evaluated in a `$derived` re-renders when a value changes). Never: email, codes, account / entry ids, anything about
// other readers. No generic property access: an unknown name is `undefined` (the text shows nothing there).
import { chapterCheckpointUri, checkpointPosition, isPrologue, type Checkpoint, type ExprValue, type MapState } from '@scutoons/content';

/** an inventory entry as far as a text may see it (no id, no code) */
export interface ReaderEntry {
	item_id: string;
	qty: number;
	source: string;
	edition_number: number | null;
}

/** what the resolver reads – getters, each optional (a missing source = no value) */
export interface ReaderSources {
	toon: string;
	/** a story variable (the toon's game state) */
	variable?(name: string): string | number | undefined;
	signedIn?(): boolean;
	/** has the reader access as a member (an account or a device code granting the catalog) */
	member?(): boolean;
	/** when the account was created (ISO / SQL date-time); none for guests */
	accountCreated?(): string | null | undefined;
	language?(): string;
	/** when the reader first read this toon (the game state's created_at) */
	startedAt?(): string | undefined;
	checkpoints?(): readonly Checkpoint[];
	/** the toon's readable chapters, in reading order */
	chapters?(): readonly number[];
	entries?(): readonly ReaderEntry[];
	/** is an item of this id worn by the avatar */
	wears?(item: string): boolean;
	/** the avatar's body (character + look) */
	body?(): { character: string; look: string } | null | undefined;
	/** how many characters the reader may wear */
	unlockedCharacters?(): number;
	/** a reader stat's current level (decayed) */
	stat?(id: string): number | undefined;
	/** the running effects' kinds */
	effects?(): readonly string[];
	map?(): MapState | null | undefined;
	/** the world map's places (with their names in the reader's language) once loaded */
	places?(): readonly { id: string; name?: string }[] | null | undefined;
	now?(): Date;
	/** the reader's name (askText scope "reader"); none = undefined */
	name?(): string | undefined;
	/** how often the event with this id ran (the history's `event:<id>`) */
	eventCount?(id: string): number;
}

const DAY = 86_400_000;

/** a date-time from the API (ISO or SQL `YYYY-MM-DD HH:MM:SS`, UTC) in ms; NaN when it isn't one */
const time = (value: string | null | undefined): number =>
	!value ? NaN : Date.parse(/[TZ+]/.test(value.slice(10)) ? value : `${value.replace(' ', 'T')}Z`);

const days = (from: string | null | undefined, now: Date): number | undefined => {
	const t = time(from);
	return Number.isNaN(t) ? undefined : Math.max(0, Math.floor((now.getTime() - t) / DAY));
};

const pad = (n: number) => String(n).padStart(2, '0');
const isoDate = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;

/** the toon's chapter checkpoint, if the reader opened it */
function checkpoint(s: ReaderSources, chapter: unknown): Checkpoint | undefined {
	const n = Number(chapter);
	if (!Number.isInteger(n)) return undefined;
	const uri = chapterCheckpointUri(s.toon, n);
	return s.checkpoints?.().find((c) => c?.uri === uri);
}

/** the unlocked places – of the reader's world map once it is loaded (so `map.total - map.unlocked` adds up), else all */
function mapPlaces(s: ReaderSources): { unlocked: number; seen?: number }[] {
	const unlocked = s.map?.()?.places ?? {};
	const world = s.places?.();
	return world ? world.flatMap((p) => (unlocked[p.id] ? [unlocked[p.id]] : [])) : Object.values(unlocked);
}

const count = (s: ReaderSources, item: unknown): number =>
	typeof item === 'string' ? (s.entries?.() ?? []).filter((e) => e.item_id === item).reduce((sum, e) => sum + e.qty, 0) : 0;

/** a name's value: `['coins']` (a story variable), `['map', 'total']` … – undefined when there is none */
export function readerName(s: ReaderSources, path: readonly string[]): ExprValue {
	if (path.length === 1) return s.variable?.(path[0]);
	if (path.length !== 2) return undefined;
	const [ns, name] = path;
	const now = s.now?.() ?? new Date();
	switch (`${ns}.${name}`) {
		// what an askText with scope "reader" asked (reader-name.svelte.ts: the account's, a guest's on the device)
		case 'reader.name':
			return s.name?.() || undefined;
		case 'reader.signedIn':
			return !!s.signedIn?.();
		case 'reader.member':
			return !!s.member?.();
		case 'reader.memberDays':
			return s.signedIn?.() ? days(s.accountCreated?.(), now) : 0;
		case 'reader.since': {
			const t = s.signedIn?.() ? time(s.accountCreated?.()) : NaN;
			return Number.isNaN(t) ? undefined : new Date(t).toISOString().slice(0, 10);
		}
		case 'reader.gifts':
			return (s.entries?.() ?? []).filter((e) => e.source === 'gift').reduce((sum, e) => sum + e.qty, 0);
		case 'reader.language':
			return s.language?.();
		case 'reader.daysSinceStart':
			return days(s.startedAt?.(), now);
		case 'read.chapters':
			return (s.checkpoints?.() ?? []).filter((c) => c?.isComplete && c.uri.startsWith(`${s.toon}/chapter/`)).length;
		case 'read.finished': {
			const chapters = (s.chapters?.() ?? []).filter((n) => !isPrologue(n));
			return chapters.length > 0 && chapters.every((n) => !!checkpoint(s, n)?.isComplete);
		}
		case 'inventory.items':
			return new Set((s.entries?.() ?? []).filter((e) => e.qty > 0).map((e) => e.item_id)).size;
		case 'avatar.character':
			return s.body?.()?.character;
		case 'avatar.look':
			return s.body?.()?.look;
		case 'avatar.unlocked':
			return s.unlockedCharacters?.();
		case 'effects.count':
			return s.effects?.().length ?? 0;
		case 'map.unlocked':
			return mapPlaces(s).length;
		case 'map.seen':
			return mapPlaces(s).filter((p) => p.seen !== undefined).length;
		case 'map.total':
			return s.places?.()?.length ?? undefined;
		case 'map.here': {
			const at = s.map?.()?.positions?.[s.toon];
			if (!at || !('location' in at)) return undefined;
			return s.places?.()?.find((p) => p.id === at.location)?.name;
		}
		case 'now.hour':
			return now.getHours();
		case 'now.weekday':
			return ((now.getDay() + 6) % 7) + 1;
		case 'now.night':
			return now.getHours() >= 22 || now.getHours() < 6;
		case 'now.date':
			return isoDate(now);
	}
	if (ns === 'stats') return s.stat?.(name);
	return undefined;
}

/** a state function's answer (`has("x")`, `inventory.count("x")`, `read.chapter(2)` …) – undefined when there is none */
export function readerCall(s: ReaderSources, name: string, args: readonly ExprValue[]): ExprValue {
	const [a] = args;
	switch (name) {
		case 'has':
		case 'inventory.has':
			return count(s, a) > 0;
		case 'count_of':
		case 'inventory.count':
			return count(s, a);
		case 'inventory.edition':
			return (s.entries?.() ?? []).find((e) => e.item_id === a && e.edition_number != null)?.edition_number ?? undefined;
		case 'read_chapter':
		case 'read.chapter':
			return !!checkpoint(s, a)?.isComplete;
		case 'read.position': {
			const cp = checkpoint(s, a);
			return cp ? checkpointPosition(cp) : 0;
		}
		case 'avatar.wears':
			return typeof a === 'string' && !!s.wears?.(a);
		case 'effects.active':
			return typeof a === 'string' && (s.effects?.() ?? []).includes(a);
		case 'map.place':
			return typeof a === 'string' && !!s.map?.()?.places?.[a];
		// an event by its id (the reader's history `event:<id>`: slice triggers, placed items' taps – EXPRESSIONS.md §5)
		case 'resolved':
			return typeof a === 'string' && (s.eventCount?.(a) ?? 0) > 0;
		case 'count':
			return typeof a === 'string' ? (s.eventCount?.(a) ?? 0) : undefined;
	}
	return undefined;
}
