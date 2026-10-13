// Wardrobe (owner 2026-10-01, agents/ITEMS.md "Wardrobe"): clothing is real, exchangeable items worn in the slots of the
// reader's avatar body (the A-pose of a look). Since 2026-10-12 worn items are NOT drawn on the body (owner: "additional
// items dont get overlayed and we just put them in the slots on the avatar page") – the avatar page shows them as icons
// in its slot tiles; every slot may be empty. Another item in the same slot instance swaps the worn one back into the
// inventory. The catalog's overlays (`ItemDef.wear`) stay as data for now (the dress-up research, research/); they only
// pick the basic dress. The server applies the same rules (AvatarService).
import type { CharacterRegistry } from './characters.ts';
import { bodySlots, defaultLookId, findLook, findPersona, REGULAR_PERSONA } from './characters.ts';
import type { ItemDef } from './items.ts';
import { AVATAR_SLOTS } from './items.ts';
import type { Problem } from './validate.ts';
import { validateAlbums } from './albums.ts';
import { READER_EFFECTS, READER_STATS } from './substances.ts';

/** `catpope`, `toptoy/legs-left` – a key of `ItemDef.wear` */
export const WEAR_KEY = /^[a-z0-9][a-z0-9-]*(\/[a-z0-9][a-z0-9-]{0,31})?$/;

/** The item has an overlay for some slot instance of this character (`<character>` or `<character>/<instance>`). */
export function hasWearFor(item: Pick<ItemDef, 'wear'> | null | undefined, character: string | null | undefined): boolean {
	if (!item?.wear || !character) return false;
	return Object.keys(item.wear).some((k) => k === character || k.startsWith(`${character}/`));
}

/**
 * The basic dress put on when the default wearables are handed out, for the body's character: per slot type of the
 * given starter wearables, the `default_worn` one with an overlay for the character; else another of that type with an
 * overlay (yellowmouse / Top Toy have no shorts overlay → the swim pants); else the `default_worn` one. Slot types without a `default_worn` item are left out (the swim pants alone are not put on). Same rule on the
 * server (AvatarService::basicDress).
 */
export function basicDress<T extends Pick<ItemDef, 'id' | 'slot' | 'wear' | 'default_worn'>>(items: T[], character: string | null | undefined): T[] {
	const out: T[] = [];
	for (const slot of AVATAR_SLOTS) {
		const ofType = items.filter((i) => i.slot === slot);
		const preferred = ofType.filter((i) => i.default_worn === true);
		if (!preferred.length) continue;
		const pick = preferred.find((i) => hasWearFor(i, character)) ?? ofType.find((i) => hasWearFor(i, character)) ?? preferred[0];
		out.push(pick);
	}
	return out;
}

const isObj = (v: unknown): v is Record<string, any> => typeof v === 'object' && v !== null && !Array.isArray(v);

/**
 * The item catalog (served by GET /api/items/; a catalog file without slots / stats – those are code,
 * AVATAR_SLOTS / READER_STATS, used when the catalog has none): ids, slots, starters and the wardrobe overlays (`wear`). With the
 * registry, `wear` keys are checked against the characters and the slot instances of their default body (the regular
 * persona's default look).
 */
export function validateCatalog(catalog: unknown, registry?: CharacterRegistry | null, file = 'items.json'): Problem[] {
	const problems: Problem[] = [];
	const add = (level: Problem['level'], path: (string | number)[], message: string) => problems.push({ level, file, locale: null, path, message });
	if (!isObj(catalog) || !Array.isArray(catalog.items)) {
		add('error', [], 'the catalog must be { slots, items: [...], recipes }');
		return problems;
	}
	const slots: string[] = Array.isArray(catalog.slots) ? catalog.slots : AVATAR_SLOTS;
	const ids = new Set<string>();
	const characters = new Map((registry?.characters ?? []).map((c) => [c.id, c]));
	catalog.items.forEach((item: any, i: number) => {
		const path = ['items', i];
		if (!isObj(item) || typeof item.id !== 'string') return add('error', path, `item ${i + 1} needs an id`);
		if (ids.has(item.id)) add('error', [...path, 'id'], `item id "${item.id}" is used twice`);
		ids.add(item.id);
		if (item.slot != null && !slots.includes(item.slot)) add('error', [...path, 'slot'], `${item.id}: unknown slot "${item.slot}"`);
		// substances (ITEMS.md "Substances"): stat changes of known stats, a timed effect, the `apply` skill
		const stats = isObj(catalog.stats) ? catalog.stats : READER_STATS;
		const applies = Array.isArray(item.skills) && item.skills.some((sk: any) => sk?.type === 'apply');
		const substance = (Array.isArray(item.effects) && item.effects.length > 0) || item.effect != null;
		if (item.effects != null) {
			if (!Array.isArray(item.effects)) add('error', [...path, 'effects'], `${item.id}: effects must be a list of { stat, delta }`);
			else item.effects.forEach((e: any, j: number) => {
				if (!isObj(e) || typeof e.stat !== 'string' || !stats[e.stat]) add('error', [...path, 'effects', j], `${item.id}: unknown stat "${e?.stat}" (catalog.stats)`);
				else if (typeof e.delta !== 'number' || !Number.isFinite(e.delta) || e.delta === 0) add('error', [...path, 'effects', j, 'delta'], `${item.id}: delta must be a number ≠ 0`);
			});
		}
		if (item.effect != null && !(isObj(item.effect) && (READER_EFFECTS as readonly unknown[]).includes(item.effect.kind) && typeof item.effect.duration_min === 'number' && item.effect.duration_min > 0)) {
			add('error', [...path, 'effect'], `${item.id}: effect must be { kind: ${READER_EFFECTS.join(' | ')}, duration_min > 0 }`);
		}
		if (item.sober != null && !(typeof item.sober === 'number' && item.sober > 0 && item.sober < 1)) add('error', [...path, 'sober'], `${item.id}: sober must be a share between 0 and 1 (0.5 halves a running effect)`);
		if (item.worn_tint != null && !(typeof item.worn_tint === 'string' && /^#[0-9a-fA-F]{6}$/.test(item.worn_tint))) add('error', [...path, 'worn_tint'], `${item.id}: worn_tint must be a colour #rrggbb`);
		else if (item.worn_tint != null && !item.slot) add('warning', [...path, 'worn_tint'], `${item.id}: worn_tint only matters for something worn (no slot)`);
		// variants with their own tint / overlays (owner 2026-10-08: the sunglasses' colours)
		(Array.isArray(item.variants) ? item.variants : []).forEach((v, vi) => {
			if (v?.worn_tint != null && !(typeof v.worn_tint === 'string' && /^#[0-9a-fA-F]{6}$/.test(v.worn_tint))) add('error', [...path, 'variants', vi, 'worn_tint'], `${item.id}/${v.id}: worn_tint must be a colour #rrggbb`);
			if (v?.wear != null && !(isObj(v.wear) && Object.values(v.wear).every((x) => typeof x === 'string' && x.trim()))) add('error', [...path, 'variants', vi, 'wear'], `${item.id}/${v.id}: wear must be { "<character>[/<slot instance>]": "<path>" }`);
		});
		if (substance !== applies) add('error', [...path, 'skills'], `${item.id}: ${substance ? 'a substance needs the "apply" skill' : 'the "apply" skill needs effects or an effect'}`);
		for (const sk of Array.isArray(item.skills) ? item.skills : []) {
			if (sk?.type !== 'scan_object') continue;
			for (const g of [sk.grants, ...(Array.isArray(sk.map) ? sk.map.map((m: any) => m?.grants) : [])]) {
				const target = catalog.items.find((x: any) => x?.id === g);
				if (!target) add('error', [...path, 'skills'], `${item.id}: scan_object grants an unknown item "${g}"`);
				else if (!Array.isArray(target.grantable) || !target.grantable.includes('scan')) add('error', [...path, 'skills'], `${item.id}: "${g}" must be grantable by "scan"`);
			}
			if (Array.isArray(sk.map) && sk.map.some((m: any) => !isObj(m) || !Array.isArray(m.labels) || !m.labels.length)) add('error', [...path, 'skills'], `${item.id}: every scan_object map entry needs labels`);
		}
		if (item.default_worn != null && (typeof item.default_worn !== 'boolean' || (item.default_worn && (item.starter !== true || !item.slot)))) {
			add('error', [...path, 'default_worn'], `${item.id}: default_worn: true is only for starter items with a slot`);
		}
		if (item.wear == null) return;
		if (!isObj(item.wear)) return add('error', [...path, 'wear'], `${item.id}: wear must be { "<character>[/<slot instance>]": "<path>" }`);
		if (!item.slot) add('error', [...path, 'wear'], `${item.id}: only wearables (with a slot) can have overlays`);
		for (const [key, value] of Object.entries(item.wear)) {
			const wp = [...path, 'wear', key];
			if (!WEAR_KEY.test(key)) {
				add('error', wp, `${item.id}: wear key "${key}" must be <character> or <character>/<slot instance>`);
				continue;
			}
			if (typeof value !== 'string' || !/\.(webp|png)$/.test(value.trim())) add('error', wp, `${item.id}: wear "${key}" must be a .webp/.png path`);
			if (!registry) continue;
			const [character, instance] = key.split('/');
			const c = characters.get(character);
			if (!c) {
				add('warning', wp, `${item.id}: unknown character "${character}"`);
				continue;
			}
			const regular = findPersona(c, REGULAR_PERSONA);
			const instances = bodySlots(findLook(regular, defaultLookId(regular))?.apose?.slots);
			if (instance) {
				const inst = instances.find((s) => s.id === instance);
				if (!inst) add('error', wp, `${item.id}: ${character} has no slot instance "${instance}"`);
				else if (item.slot && inst.slot !== item.slot) add('error', wp, `${item.id}: ${character}/${instance} is a ${inst.slot} slot, the item is worn on ${item.slot}`);
			} else if (item.slot && instances.filter((s) => s.slot === item.slot).length > 1) {
				add('warning', wp, `${item.id}: ${character} has several ${item.slot} slots – give one overlay per slot instance`);
			}
		}
	});
	// albums, formats and players (albums.ts): a record plays only with a player for its format
	validateAlbums(catalog, add);
	for (const [id, def] of Object.entries(isObj(catalog.stats) ? catalog.stats : {})) {
		const d = def as any;
		if (!isObj(d) || typeof d.baseline !== 'number' || d.baseline < 0 || d.baseline > 100 || typeof d.half_life_days !== 'number' || d.half_life_days <= 0) {
			add('error', ['stats', id], `stat "${id}" must be { label, baseline 0–100, half_life_days > 0 }`);
		}
	}
	return problems;
}
