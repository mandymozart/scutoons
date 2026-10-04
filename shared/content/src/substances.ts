// Substances & reader stats (owner 2026-10-01, agents/ITEMS.md "Substances"). Game mechanics only: no real-world
// dosing, use instructions or claims about real effects anywhere in the texts.
//
// - There is no male/female for anyone. Characters (and personas) may carry a `testo` level 0–100 instead (character
//   data, no visual effect yet: `testoBand` is the hook for future A-pose / portrait variants); the reader has one too.
// - A `substance` item (consumable, skill `apply`) changes **reader stats** (`effects: [{ stat, delta }]`, e.g. testo +20,
//   appetite −15) and/or starts a temporary **reader effect** (`effect: { kind, duration_min }`, e.g. `psychedelic`).
// - A stat drifts back to its baseline with a half-life (`catalog.stats`: `{ baseline, half_life_days, label }`), computed
//   lazily from `{ level, updated_at }` – no cron. The server stores the same and applies the same math
//   (Domain\Reader\ReaderState); guests keep it in localStorage.
import type { Catalog, ItemDef, Localized, ScanObjectSkill } from './items.ts';
import { AVATAR_SLOTS } from './items.ts';

export const STAT_MIN = 0;
export const STAT_MAX = 100;
/** the reader's testo stat id */
export const TESTO_STAT = 'testo';

/** A reader stat (`catalog.stats.<id>`): where it rests and how fast it gets back there. */
export interface StatDef {
	label: Localized;
	/** 0–100, the neutral start and where the level drifts back to */
	baseline: number;
	/** the distance to the baseline halves every N days */
	half_life_days: number;
}

/** One stat change of a substance. */
export interface StatEffect {
	stat: string;
	/** added to the current (decayed) level, then clamped to 0–100 */
	delta: number;
}

/** A temporary reader effect of a substance (e.g. `psychedelic`): active until now + duration. */
export interface TimedEffect {
	kind: string;
	duration_min: number;
}

/**
 * The reader stats (code, not catalog data – owner 2026-10-04, agents/CATALOG.md): the server serves the same as
 * `stats` in GET /api/items/ (server Domain\Reader\ReaderStats; its CatalogTest keeps both equal).
 */
export const READER_STATS: Record<string, StatDef> = {
	testo: { label: { de: 'Testo', en: 'Testo' }, baseline: 50, half_life_days: 3 },
	appetite: { label: { de: 'Appetit', en: 'Appetite' }, baseline: 50, half_life_days: 3 }
};

/**
 * A catalog in the served shape (GET /api/items/) from the server's seed file `server/config/items.json`, which no
 * longer carries the code parts: `slots` (AVATAR_SLOTS) and `stats` (READER_STATS) are added – for tests and scripts.
 */
export function catalogFromSeed(seed: Omit<Catalog, 'slots' | 'stats'> & Partial<Pick<Catalog, 'slots' | 'stats'>>): Catalog {
	const { version, items, recipes, albums } = seed;
	return { version, slots: seed.slots ?? [...AVATAR_SLOTS], items, recipes, ...(albums ? { albums } : {}), stats: seed.stats ?? READER_STATS };
}

/** A stored stat: the level at `updated_at` (ISO); the current level is `statLevel` of it. */
export interface StatState {
	level: number;
	updated_at: string;
}

export interface ActiveEffect {
	kind: string;
	/** ISO end time */
	until: string;
}

/** What the reader carries: stats (stored levels) and active effects (`/me/ stats`, `/me/ effects`, guests: localStorage). */
export interface ReaderState {
	stats: Record<string, StatState>;
	effects: ActiveEffect[];
}

export const clampStat = (v: number) => Math.min(STAT_MAX, Math.max(STAT_MIN, v));

/** The level now: the stored level drifted back toward the baseline (half-life), or the baseline when nothing is stored. */
export function statLevel(def: Pick<StatDef, 'baseline' | 'half_life_days'>, state: StatState | null | undefined, now = Date.now()): number {
	if (!state) return def.baseline;
	const days = Math.max(0, now - Date.parse(state.updated_at)) / 86_400_000;
	const factor = def.half_life_days > 0 ? Math.pow(0.5, days / def.half_life_days) : 0;
	return clampStat(def.baseline + (state.level - def.baseline) * factor);
}

/** Every stat of the catalog with its current level (rounded for display). */
export function statLevels(catalog: Pick<Catalog, 'stats'> | null | undefined, state: ReaderState | null | undefined, now = Date.now()): { id: string; def: StatDef; level: number }[] {
	return Object.entries(catalog?.stats ?? {}).map(([id, def]) => ({ id, def, level: Math.round(statLevel(def, state?.stats?.[id], now)) }));
}

/** Effects still running at `now`. */
export function activeEffects(effects: ActiveEffect[] | null | undefined, now = Date.now()): ActiveEffect[] {
	return (effects ?? []).filter((e) => Date.parse(e.until) > now);
}

/** An item that can be applied (`apply` skill with stat changes or a timed effect). */
export function isSubstance(item: Pick<ItemDef, 'effects' | 'effect'> | null | undefined): boolean {
	return !!item && ((item.effects?.length ?? 0) > 0 || !!item.effect);
}

/**
 * Applies one piece of a substance: every stat effect moves the current (decayed) level by its delta (clamped 0–100,
 * stored with `now`), a timed effect runs until now + its duration (a running one of the same kind is replaced).
 * Same math on the server (ReaderState::apply).
 */
export function applySubstance(state: ReaderState, item: Pick<ItemDef, 'effects' | 'effect'>, stats: Record<string, StatDef>, now = Date.now()): ReaderState {
	const at = new Date(now).toISOString();
	const next: ReaderState = { stats: { ...state.stats }, effects: activeEffects(state.effects, now) };
	for (const e of item.effects ?? []) {
		const def = stats[e.stat];
		if (!def) continue;
		next.stats[e.stat] = { level: clampStat(statLevel(def, state.stats[e.stat], now) + e.delta), updated_at: at };
	}
	if (item.effect) {
		next.effects = [...next.effects.filter((x) => x.kind !== item.effect!.kind), { kind: item.effect.kind, until: new Date(now + item.effect.duration_min * 60_000).toISOString() }];
	}
	return next;
}

/** Ends a running effect ("Effekt beenden"). */
export function endEffect(state: ReaderState, kind: string): ReaderState {
	return { ...state, effects: state.effects.filter((e) => e.kind !== kind) };
}

/**
 * The band of a testo level – the hook for future A-pose / portrait variants (no visual change yet). Direction (owner
 * 2026-10-01): more testo → the character reads more male, less testo → more female: `testo-low` = female-read,
 * `testo-mid` = ambiguous, `testo-high` = male-read; the art variants follow that later.
 */
export type TestoBand = 'testo-low' | 'testo-mid' | 'testo-high';
export function testoBand(level: number): TestoBand {
	return level < 34 ? 'testo-low' : level < 67 ? 'testo-mid' : 'testo-high';
}

/** A character's (persona's) testo level: the persona's own, else the character's; undefined = not set. */
export function characterTesto(character: { testo?: number; personas?: { id: string; testo?: number }[] } | null | undefined, persona?: string): number | undefined {
	const own = persona ? character?.personas?.find((p) => p.id === persona)?.testo : undefined;
	return own ?? character?.testo;
}

/** One thing a `scan_object` camera looks for: model labels → the item granted (and what to call it). */
export interface ScanTarget {
	labels: string[];
	grants: string;
	name?: Localized;
}

/** Every target of a `scan_object` skill: `labels` → `grants`, then each `map` entry. */
export function scanTargets(skill: Pick<ScanObjectSkill, 'labels' | 'grants' | 'name' | 'map'>): ScanTarget[] {
	return [{ labels: skill.labels ?? [], grants: skill.grants, name: skill.name }, ...(skill.map ?? [])].filter((t) => t.grants && t.labels?.length);
}

/**
 * The best prediction that matches any target (ImageNet class names are comma lists like "sunscreen, sunblock, sun
 * blocker": a label matches one of the parts, or one word of a part). Game recognition only – never an analysis.
 */
export function matchScan(
	predictions: { className: string; probability: number }[],
	skill: Pick<ScanObjectSkill, 'labels' | 'grants' | 'name' | 'map'>
): { target: ScanTarget; label: string; probability: number } | null {
	let best: { target: ScanTarget; label: string; probability: number } | null = null;
	for (const target of scanTargets(skill)) {
		const wanted = target.labels.map((l) => l.toLowerCase());
		for (const p of predictions) {
			const parts = p.className.toLowerCase().split(',').map((x) => x.trim());
			const label = wanted.find((w) => parts.some((part) => part === w || part.split(/\s+/).includes(w)));
			if (label && (!best || p.probability > best.probability)) best = { target, label, probability: p.probability };
		}
	}
	return best;
}

/** A frame's score for one `scan_object` target: the summed probability of its matching classes, and its best rank. */
export interface ScanScore {
	target: ScanTarget;
	label: string;
	/** sum of the probabilities of every prediction that matches the target (cucumber + zucchini count together) */
	score: number;
	/** 1-based rank of the target's best matching prediction among all predictions */
	rank: number;
}

/**
 * The best scoring target of one frame (owner test 2026-10-02: a real cucumber on a floor came back as "slug 0.35 |
 * quill 0.14 | cucumber 0.13" – the cucumber was there, but never alone above a fixed threshold). A frame passes when
 * the summed score reaches `threshold`, or when the target is among the `top` predictions with at least `min`; the
 * camera still needs several passing frames in a row before it grants anything.
 */
export function scoreScan(
	predictions: { className: string; probability: number }[],
	skill: Pick<ScanObjectSkill, 'labels' | 'grants' | 'name' | 'map'>
): ScanScore | null {
	const sorted = [...predictions].sort((a, b) => b.probability - a.probability);
	let best: ScanScore | null = null;
	for (const target of scanTargets(skill)) {
		const wanted = target.labels.map((l) => l.toLowerCase());
		let score = 0;
		let rank = 0;
		let label = '';
		sorted.forEach((p, i) => {
			const parts = p.className.toLowerCase().split(',').map((x) => x.trim());
			const hit = wanted.find((w) => parts.some((part) => part === w || part.split(/\s+/).includes(w)));
			if (!hit) return;
			score += p.probability;
			if (!rank) (rank = i + 1), (label = hit);
		});
		if (rank && (!best || score > best.score)) best = { target, label, score, rank };
	}
	return best;
}

/** Does a frame's score pass (see `scoreScan`)? */
export function scanPasses(hit: ScanScore | null, rule: { threshold: number; top: number; min: number }): boolean {
	return !!hit && (hit.score >= rule.threshold || (hit.rank <= rule.top && hit.score >= rule.min));
}
