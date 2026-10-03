// When a picked-up item is back on its slice (agents/ITEMS.md "Placed items" → "Respawn"). Two levels (owner 2026-10-03):
// the ITEM (catalog) says whether it can reappear at all, its default interval and the range a placement may choose;
// the PLACED ITEM (`SliceItem.respawn`) may override within that range. The server mirrors these rules
// (server/src/Domain/Catalog/ItemRespawn.php) and enforces them: the grant ref gets the period (visit id / time bucket).

/** Every visit of the page (a reload counts), or every n minutes / hours / days. */
export type RespawnInterval = 'visit' | { minutes: number } | { hours: number } | { days: number };

/**
 * An item's respawn rule (`ItemDef.respawn`; none / null = it never reappears, a one-time find). `min` / `max` = the
 * shortest / longest interval a placement may choose (default: only `default`'s own length … any longer one).
 */
export interface ItemRespawn {
	default: RespawnInterval;
	min?: RespawnInterval;
	max?: RespawnInterval;
}

/** A placement's choice: `never` = once even if the item could reappear; none = the item's default */
export type SliceItemRespawn = RespawnInterval | 'never';

/** the unit of a timed interval */
export const RESPAWN_UNITS = ['minutes', 'hours', 'days'] as const;
export type RespawnUnit = (typeof RESPAWN_UNITS)[number];
const UNIT_MINUTES: Record<RespawnUnit, number> = { minutes: 1, hours: 60, days: 1440 };
/** the longest interval: a year */
export const RESPAWN_MAX_MINUTES = 365 * 1440;

/** an interval's length in minutes (visit = 0), null when it isn't one */
export function intervalMinutes(value: unknown): number | null {
	if (value === 'visit') return 0;
	if (!value || typeof value !== 'object' || Array.isArray(value)) return null;
	const keys = Object.keys(value);
	if (keys.length !== 1) return null;
	const unit = keys[0] as RespawnUnit;
	const n = (value as Record<string, unknown>)[unit];
	if (!(unit in UNIT_MINUTES) || typeof n !== 'number' || !Number.isInteger(n) || n < 1) return null;
	const minutes = n * UNIT_MINUTES[unit];
	return minutes <= RESPAWN_MAX_MINUTES ? minutes : null;
}

export const isRespawnInterval = (value: unknown): value is RespawnInterval => intervalMinutes(value) !== null;

/** `{ unit, n }` of an interval (visit: unit 'visit') */
export function intervalParts(value: RespawnInterval): { unit: 'visit' | RespawnUnit; n: number } {
	if (value === 'visit') return { unit: 'visit', n: 0 };
	const unit = Object.keys(value)[0] as RespawnUnit;
	return { unit, n: (value as Record<string, number>)[unit] };
}

/** Problems of an item rule (catalog / admin), empty = fine */
export function itemRespawnProblems(rule: unknown): string[] {
	if (rule === undefined || rule === null) return [];
	if (typeof rule !== 'object' || Array.isArray(rule)) return ['respawn must be { default, min?, max? } or null (never)'];
	const r = rule as Record<string, unknown>;
	const out: string[] = [];
	const d = intervalMinutes(r.default);
	if (d === null) out.push('respawn.default must be "visit" or { minutes | hours | days: n }');
	const min = r.min === undefined ? d : intervalMinutes(r.min);
	const max = r.max === undefined ? RESPAWN_MAX_MINUTES : intervalMinutes(r.max);
	if (r.min !== undefined && min === null) out.push('respawn.min must be "visit" or { minutes | hours | days: n }');
	if (r.max !== undefined && max === null) out.push('respawn.max must be "visit" or { minutes | hours | days: n }');
	if (d !== null && min !== null && max !== null && !(min <= d && d <= max)) out.push('respawn: min ≤ default ≤ max');
	for (const key of Object.keys(r)) if (!['default', 'min', 'max'].includes(key)) out.push(`respawn.${key} is unknown`);
	return out;
}

/** the range (minutes) a placement may choose from an item rule */
export function respawnRange(rule: ItemRespawn): { min: number; max: number } {
	const d = intervalMinutes(rule.default) ?? 0;
	return {
		min: rule.min === undefined ? d : (intervalMinutes(rule.min) ?? d),
		max: rule.max === undefined ? RESPAWN_MAX_MINUTES : (intervalMinutes(rule.max) ?? RESPAWN_MAX_MINUTES)
	};
}

/**
 * The interval a placement reappears with: its own choice within the item's rule, else the item's default; null = never
 * (the item has no rule, or the placement says `never`). An out-of-range choice falls back to the item's default.
 */
export function effectiveRespawn(choice: SliceItemRespawn | undefined, rule: ItemRespawn | null | undefined): RespawnInterval | null {
	if (!rule || !isRespawnInterval(rule.default) || choice === 'never') return null;
	if (choice === undefined) return rule.default;
	const minutes = intervalMinutes(choice);
	if (minutes === null) return rule.default;
	const { min, max } = respawnRange(rule);
	return minutes >= min && minutes <= max ? choice : rule.default;
}

/** Problems of a placement's choice against the item's rule (`rule` undefined = catalog not known, only the shape) */
export function placementRespawnProblem(choice: unknown, rule: ItemRespawn | null | undefined): string | null {
	if (choice === undefined || choice === 'never') return null;
	const minutes = intervalMinutes(choice);
	if (minutes === null) return 'respawn must be "never", "visit" or { minutes | hours | days: n }';
	if (rule === undefined) return null;
	if (!rule) return 'this item never reappears (catalog) – use "never" or remove respawn';
	const { min, max } = respawnRange(rule);
	if (minutes < min) return 'shorter than the item allows (catalog respawn.min)';
	if (minutes > max) return 'longer than the item allows (catalog respawn.max)';
	return null;
}

/** The period a timed interval is in (`m<minutes>:<bucket>`; fixed buckets since 1970, UTC); visit → `v:<visit id>` */
export function respawnPeriod(interval: RespawnInterval, now: number, visitId: string): string {
	const minutes = intervalMinutes(interval) ?? 0;
	if (minutes === 0) return `v:${visitId}`;
	return `m${minutes}:${Math.floor(now / (minutes * 60_000))}`;
}
