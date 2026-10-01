// Wardrobe (owner 2026-10-01, agents/ITEMS.md "Wardrobe"): clothing is real, exchangeable items drawn as full-body
// overlays on the reader's avatar body (a character's A-pose). Every reader starts with the SCUTOONS T-shirt and swim
// pants worn; another item in the same slot instance swaps the worn one back into the inventory. While "Hide sensitive
// content" is on (members: by default, guests: always) a body never shows naked: the cover slots (`legs`) are refilled
// from the inventory (swim pants first) and – where nothing is left to wear – the character's safe look (the swim
// A-pose with the pants painted in) is the base image. The server applies the same rules (AvatarService).
import type { AvatarBody, AvatarSlotInstance, CharacterRegistry } from './characters.ts';
import { bodySlots } from './characters.ts';
import type { AvatarSlot, Catalog, ItemDef } from './items.ts';
import { AVATAR_SLOTS } from './items.ts';
import type { Problem } from './validate.ts';

/**
 * Draw order of the overlays, bottom → top (legs under the shoes, the shirt over the waistband, the backpack over the
 * shirt, then hands, neck, head, face). Items without an overlay for the body are drawn after all overlays, as icons at
 * their slot anchors.
 */
export const WEAR_ORDER: AvatarSlot[] = ['legs', 'feet', 'torso', 'back', 'hands', 'accessory', 'neck', 'head', 'face'];

/** Slot types that must be covered while "Hide sensitive content" is on (a naked body would show otherwise). */
export const SENSITIVE_COVER: AvatarSlot[] = ['legs'];

/** `catpope`, `toptoy/legs-left` – a key of `ItemDef.wear` */
export const WEAR_KEY = /^[a-z0-9][a-z0-9-]*(\/[a-z0-9][a-z0-9-]{0,31})?$/;

/** Folder of a character's overlays (from the CDN root). */
export const wearFolder = (character: string) => `characters/${character}/wear`;

/**
 * Convention path of an overlay: `characters/<character>/wear/<item>.webp` for a single figure (the instance id equals
 * its slot type), `…/<item>-<instance id>.webp` for one figure of a multi-figure body (Top Toy `legs-left`).
 */
export function wearConventionPath(character: string, item: string, instance?: Pick<AvatarSlotInstance, 'id' | 'slot'> | null): string {
	const suffix = instance && instance.id !== instance.slot ? `-${instance.id}` : '';
	return `${wearFolder(character)}/${item}${suffix}.webp`;
}

/** The overlay of an item on a character's slot instance: `<character>/<instance>` first, then `<character>`; null = none. */
export function wearOverlay(item: Pick<ItemDef, 'wear'> | null | undefined, character: string | null | undefined, instanceId: string): string | null {
	const wear = item?.wear;
	if (!wear || !character) return null;
	const path = wear[`${character}/${instanceId}`] ?? wear[character];
	return typeof path === 'string' && path.trim() ? path.trim() : null;
}

/** The item has an overlay for some slot instance of this character (`<character>` or `<character>/<instance>`). */
export function hasWearFor(item: Pick<ItemDef, 'wear'> | null | undefined, character: string | null | undefined): boolean {
	if (!item?.wear || !character) return false;
	return Object.keys(item.wear).some((k) => k === character || k.startsWith(`${character}/`));
}

/**
 * The basic dress put on when the default wearables are handed out, for the body's character: per slot type of the
 * given starter wearables, the `default_worn` one with an overlay for the character; else another of that type with an
 * overlay (yellowmouse / Top Toy have no shorts overlay → the swim pants); else the `default_worn` one (drawn as an
 * icon). Slot types without a `default_worn` item are left out (the swim pants alone are not put on). Same rule on the
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

/**
 * Fills the empty cover-slot instances of a body (SENSITIVE_COVER) with owned, unworn entries – in the order given
 * (the caller passes the inventory in catalog order, so the swim pants come first). Instances stay empty when nothing
 * is left (twins with one pair of pants: the safe base covers that figure). Returns the new worn map and the filled ids.
 */
export function coverSensitive<E extends string | number>(
	worn: Record<string, E | null | undefined>,
	instances: Pick<AvatarSlotInstance, 'id' | 'slot'>[],
	candidates: { id: E; slot: AvatarSlot | null | undefined }[]
): { worn: Record<string, E | null>; restored: string[] } {
	const out: Record<string, E | null> = {};
	for (const [k, v] of Object.entries(worn)) out[k] = v ?? null;
	const used = new Set(Object.values(out).filter((v) => v != null).map(String));
	const restored: string[] = [];
	for (const inst of instances) {
		if (!SENSITIVE_COVER.includes(inst.slot) || out[inst.id] != null) continue;
		const next = candidates.find((c) => c.slot === inst.slot && !used.has(String(c.id)));
		if (!next) continue;
		out[inst.id] = next.id;
		used.add(String(next.id));
		restored.push(inst.id);
	}
	return { worn: out, restored };
}

/**
 * The base image of the avatar: the character's sensitive (naked) look when the reader may see it – setting off, or
 * every cover instance wears an item with an overlay for this character –, else its safe look (default, non-sensitive).
 * `bodies` = the reader's unlocked bodies *including* sensitive ones; `body` = the shown (safe) body.
 */
export function wardrobeBase(
	bodies: AvatarBody[],
	body: AvatarBody | null | undefined,
	opts: { hide: boolean; covered: boolean }
): { path: string; naked: boolean } | null {
	if (!body) return null;
	const naked = bodies.find((b) => b.character === body.character && b.sensitive);
	if (naked && (!opts.hide || opts.covered)) return { path: naked.path, naked: true };
	const safe = body.sensitive ? (bodies.find((b) => b.character === body.character && !b.sensitive) ?? null) : body;
	return safe ? { path: safe.path, naked: false } : null;
}

/** A wearable × body slot instance without an overlay (an asset request for the owner; the icon is drawn meanwhile). */
export interface MissingWear {
	item: string;
	character: string;
	instance: string;
	/** where the overlay should go (convention, from the CDN root) */
	path: string;
}

/**
 * Every wearable item (with a slot) on every A-pose body of the registry (the default look's slot instances of the
 * item's type) that has no overlay yet.
 */
export function missingWearOverlays(catalog: Pick<Catalog, 'items'> | null | undefined, registry: CharacterRegistry | null | undefined): MissingWear[] {
	const out: MissingWear[] = [];
	for (const item of catalog?.items ?? []) {
		if (!item.slot) continue;
		for (const c of registry?.characters ?? []) {
			const looks = c?.apose?.looks ?? [];
			const look = looks.find((l) => l?.default) ?? looks[0];
			if (!look?.path?.trim()) continue;
			for (const inst of bodySlots(look.slots).filter((s) => s.slot === item.slot)) {
				if (!wearOverlay(item, c.id, inst.id)) out.push({ item: item.id, character: c.id, instance: inst.id, path: wearConventionPath(c.id, item.id, inst) });
			}
		}
	}
	return out;
}

const isObj = (v: unknown): v is Record<string, any> => typeof v === 'object' && v !== null && !Array.isArray(v);

/**
 * The item catalog (server `config/items.json`): ids, slots, starters and the wardrobe overlays (`wear`). With the
 * registry, `wear` keys are checked against the characters and their slot instances.
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
		const stats = isObj(catalog.stats) ? catalog.stats : {};
		const applies = Array.isArray(item.skills) && item.skills.some((sk: any) => sk?.type === 'apply');
		const substance = (Array.isArray(item.effects) && item.effects.length > 0) || item.effect != null;
		if (item.effects != null) {
			if (!Array.isArray(item.effects)) add('error', [...path, 'effects'], `${item.id}: effects must be a list of { stat, delta }`);
			else item.effects.forEach((e: any, j: number) => {
				if (!isObj(e) || typeof e.stat !== 'string' || !stats[e.stat]) add('error', [...path, 'effects', j], `${item.id}: unknown stat "${e?.stat}" (catalog.stats)`);
				else if (typeof e.delta !== 'number' || !Number.isFinite(e.delta) || e.delta === 0) add('error', [...path, 'effects', j, 'delta'], `${item.id}: delta must be a number ≠ 0`);
			});
		}
		if (item.effect != null && !(isObj(item.effect) && typeof item.effect.kind === 'string' && /^[a-z0-9][a-z0-9_-]*$/.test(item.effect.kind) && typeof item.effect.duration_min === 'number' && item.effect.duration_min > 0)) {
			add('error', [...path, 'effect'], `${item.id}: effect must be { kind, duration_min > 0 }`);
		}
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
			const looks = c.apose?.looks ?? [];
			const instances = bodySlots((looks.find((l) => l.default) ?? looks[0])?.slots);
			if (instance) {
				const inst = instances.find((s) => s.id === instance);
				if (!inst) add('error', wp, `${item.id}: ${character} has no slot instance "${instance}"`);
				else if (item.slot && inst.slot !== item.slot) add('error', wp, `${item.id}: ${character}/${instance} is a ${inst.slot} slot, the item is worn on ${item.slot}`);
			} else if (item.slot && instances.filter((s) => s.slot === item.slot).length > 1) {
				add('warning', wp, `${item.id}: ${character} has several ${item.slot} slots – give one overlay per slot instance`);
			}
		}
	});
	for (const [id, def] of Object.entries(isObj(catalog.stats) ? catalog.stats : {})) {
		const d = def as any;
		if (!isObj(d) || typeof d.baseline !== 'number' || d.baseline < 0 || d.baseline > 100 || typeof d.half_life_days !== 'number' || d.half_life_days <= 0) {
			add('error', ['stats', id], `stat "${id}" must be { label, baseline 0–100, half_life_days > 0 }`);
		}
	}
		for (const slot of SENSITIVE_COVER) {
		if (!catalog.items.some((it: any) => isObj(it) && it.starter === true && it.slot === slot)) {
			add('warning', ['items'], `no starter item for the "${slot}" slot: "Hide sensitive content" can't keep the avatar covered`);
		}
	}
	return problems;
}
