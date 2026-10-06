// When a picked-up item is back on its slice (agents/ITEMS.md "Placed items" → "Respawn"). Two levels (owner 2026-10-03):
// the ITEM (catalog) says whether it can reappear at all, its default interval and the range a placement may choose;
// the PLACED ITEM (`SliceItem.respawn`) may override within that range. The server mirrors these rules
// (server/src/Domain/Catalog/ItemRespawn.php) and enforces them: the grant ref gets the period (recurrence.ts `periodOf`:
// the visit id / time bucket).

import { intervalMinutes, isRecurrenceInterval, RECURRENCE_MAX_MINUTES, type RecurrenceInterval } from './recurrence.ts';

/**
 * An item's respawn rule (`ItemDef.respawn`; none / null = it never reappears, a one-time find). `min` / `max` = the
 * shortest / longest interval a placement may choose (default: only `default`'s own length … any longer one). The
 * intervals and their periods are the shared recurrence model (recurrence.ts).
 */
export interface ItemRespawn {
	default: RecurrenceInterval;
	min?: RecurrenceInterval;
	max?: RecurrenceInterval;
}

/** A placement's choice: `never` = once even if the item could reappear; none = the item's default */
export type SliceItemRespawn = RecurrenceInterval | 'never';

/** Problems of an item rule (catalog / admin), empty = fine */
export function itemRespawnProblems(rule: unknown): string[] {
	if (rule === undefined || rule === null) return [];
	if (typeof rule !== 'object' || Array.isArray(rule)) return ['respawn must be { default, min?, max? } or null (never)'];
	const r = rule as Record<string, unknown>;
	const out: string[] = [];
	const d = intervalMinutes(r.default);
	if (d === null) out.push('respawn.default must be "visit" or { minutes | hours | days: n }');
	const min = r.min === undefined ? d : intervalMinutes(r.min);
	const max = r.max === undefined ? RECURRENCE_MAX_MINUTES : intervalMinutes(r.max);
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
		max: rule.max === undefined ? RECURRENCE_MAX_MINUTES : (intervalMinutes(rule.max) ?? RECURRENCE_MAX_MINUTES)
	};
}

/**
 * The interval a placement reappears with: its own choice within the item's rule, else the item's default; null = never
 * (the item has no rule, or the placement says `never`). An out-of-range choice falls back to the item's default.
 */
export function effectiveRespawn(choice: SliceItemRespawn | undefined, rule: ItemRespawn | null | undefined): RecurrenceInterval | null {
	if (!rule || !isRecurrenceInterval(rule.default) || choice === 'never') return null;
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
