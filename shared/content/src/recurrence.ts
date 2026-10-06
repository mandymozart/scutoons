// One recurrence model for placed items AND events (agents/EXPRESSIONS.md §5, owner 2026-10-06: "recycle the item pickup
// logic, so this timing is not implemented twice"). Pure: the intervals (every page view, every n minutes / hours / days),
// the periods they make (fixed time buckets since 1970, UTC – the server's respawn check uses the same buckets), and the
// repeat rule of an event (`repeat`: every time, once per page view, once for good, every n …) decided against the
// reader's history entry (history.ts – one shape `{ n, first, last }` for everything a reader met).
//
// Placed items (pickups, respawn.ts): the catalog's rule + the placement's choice give an interval; the period is
// remembered in the game state and the server grants once per period (its once-per-ref grant stays the authority).
// Events (slice triggers, item taps): `repeat` + the history entry `event:<id>` say whether they run now.

/** A timed interval: every n minutes / hours / days (n a whole number ≥ 1, at most a year). */
export type TimedInterval = { minutes: number } | { hours: number } | { days: number };

/** Every page view (`visit`: a new page or a reload starts a new one), or every n minutes / hours / days. */
export type RecurrenceInterval = 'visit' | TimedInterval;

/** the unit of a timed interval */
export const RECURRENCE_UNITS = ['minutes', 'hours', 'days'] as const;
export type RecurrenceUnit = (typeof RECURRENCE_UNITS)[number];
const UNIT_MINUTES: Record<RecurrenceUnit, number> = { minutes: 1, hours: 60, days: 1440 };
/** the longest interval: a year */
export const RECURRENCE_MAX_MINUTES = 365 * 1440;

/** an interval's length in minutes (visit = 0), null when it isn't one */
export function intervalMinutes(value: unknown): number | null {
	if (value === 'visit') return 0;
	if (!value || typeof value !== 'object' || Array.isArray(value)) return null;
	const keys = Object.keys(value);
	if (keys.length !== 1) return null;
	const unit = keys[0] as RecurrenceUnit;
	const n = (value as Record<string, unknown>)[unit];
	if (!(unit in UNIT_MINUTES) || typeof n !== 'number' || !Number.isInteger(n) || n < 1) return null;
	const minutes = n * UNIT_MINUTES[unit];
	return minutes <= RECURRENCE_MAX_MINUTES ? minutes : null;
}

export const isRecurrenceInterval = (value: unknown): value is RecurrenceInterval => intervalMinutes(value) !== null;
export const isTimedInterval = (value: unknown): value is TimedInterval => (intervalMinutes(value) ?? 0) > 0;

/** `{ unit, n }` of an interval (visit: unit 'visit') */
export function intervalParts(value: RecurrenceInterval): { unit: 'visit' | RecurrenceUnit; n: number } {
	if (value === 'visit') return { unit: 'visit', n: 0 };
	const unit = Object.keys(value)[0] as RecurrenceUnit;
	return { unit, n: (value as Record<string, number>)[unit] };
}

/** The period `now` is in: a timed interval's fixed bucket since 1970, UTC (`m<minutes>:<bucket>`); visit → `v:<view id>` */
export function periodOf(interval: RecurrenceInterval, now: number, viewId: string): string {
	const minutes = intervalMinutes(interval) ?? 0;
	if (minutes === 0) return `v:${viewId}`;
	return `m${minutes}:${Math.floor(now / (minutes * 60_000))}`;
}

// ── events: the repeat rule ───────────────────────────────────────────────────────────────────────────────────

/**
 * How often an event runs (`VNEventTrigger.repeat`, `SliceItem.repeat` for a placed item's tap events):
 * - `every` – every time its trigger fires (the default: no `repeat`), e.g. most onomatopoeia;
 * - `view` – once per page view (a new page or a reload is a new one);
 * - `once` – once for good (checked off in the reader's save game, never again);
 * - `interval` – once per period of `every` (n minutes / hours / days; the same fixed buckets as an item's respawn).
 * "When a condition holds" is `every` with the event's `if`.
 */
export const REPEAT_MODES = ['every', 'view', 'once', 'interval'] as const;
export type RepeatMode = (typeof REPEAT_MODES)[number];

export interface Repeat {
	mode: RepeatMode;
	/** `interval` only: the period */
	every?: TimedInterval;
}

/** the rule a stored `repeat` stands for: none (or one that isn't a rule) = every time */
export function repeatOf(value: unknown): Repeat {
	if (!repeatProblem(value) && value) return value as Repeat;
	return { mode: 'every' };
}

/** what is wrong with a stored `repeat` (null = fine; undefined = none = every time) */
export function repeatProblem(value: unknown): string | null {
	if (value === undefined) return null;
	if (!value || typeof value !== 'object' || Array.isArray(value)) return 'repeat must be { mode: every | view | once | interval, every? }';
	const r = value as Record<string, unknown>;
	if (!(REPEAT_MODES as readonly unknown[]).includes(r.mode)) return `repeat.mode must be one of ${REPEAT_MODES.join(', ')}`;
	for (const key of Object.keys(r)) if (key !== 'mode' && key !== 'every') return `repeat.${key} is unknown`;
	if (r.mode === 'interval') {
		if (!isTimedInterval(r.every)) return 'repeat every … needs every: { minutes | hours | days: n } (n ≥ 1, at most a year)';
	} else if (r.every !== undefined) return 'repeat.every belongs to mode interval';
	return null;
}

/** the history an event's decision reads: how often it ran and when last (history.ts `HistoryEntry`) */
export interface RecurrenceEntry {
	n: number;
	last: number;
}

/** the page view the reader is on: its id and when it started (ms since 1970) */
export interface PageView {
	id: string;
	start: number;
}

/**
 * Does an event with this rule run now? `entry` = its history (none = never ran), `view` = the current page view. A run
 * that is due is recorded in the history by the caller (history.ts `recordHistory`), which is what makes the next
 * decision.
 */
export function isDue(rule: Repeat | undefined, entry: RecurrenceEntry | null | undefined, now: number, view: PageView): boolean {
	const r = repeatOf(rule);
	if (r.mode === 'every' || !entry || !(entry.n > 0)) return true;
	switch (r.mode) {
		case 'view':
			return entry.last < view.start;
		case 'once':
			return false;
		case 'interval':
			return periodOf(r.every!, entry.last, view.id) !== periodOf(r.every!, now, view.id);
	}
	return true;
}

/** a rule that never needs the save game (decided within the page view): every time / once per page view */
export const isViewRule = (rule: Repeat | undefined): boolean => repeatOf(rule).mode === 'every' || repeatOf(rule).mode === 'view';

/**
 * A placed item's pickup in the same terms (the creator's one "Repeats" field): never reappears = once for good, every
 * visit = once per page view, a timed interval = once per period.
 */
export function respawnAsRepeat(interval: RecurrenceInterval | null): Repeat {
	if (interval === null) return { mode: 'once' };
	if (interval === 'visit') return { mode: 'view' };
	return { mode: 'interval', every: interval };
}

/** …and back (`every` has no pickup form: null) */
export function repeatAsRespawn(rule: Repeat): RecurrenceInterval | 'never' | null {
	if (rule.mode === 'once') return 'never';
	if (rule.mode === 'view') return 'visit';
	if (rule.mode === 'interval' && rule.every) return rule.every;
	return null;
}
