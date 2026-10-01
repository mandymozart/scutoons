import { setContext, getContext } from 'svelte';
import { ApiError } from '$lib/api.ts';
import {
	claimCode,
	craftRecipe,
	fetchCatalog,
	fetchEditions,
	grantItem,
	importItems,
	putAvatar,
	putAvatarBody,
	fetchRegistry,
	useItem,
	endEffect as endEffectApi
} from '$lib/api.ts';
import {
	DEFAULT_AVATAR_UNLOCKS,
	SENSITIVE_COVER,
	activeEffects,
	applySubstance,
	avatarBodies,
	endEffect as endEffectState,
	isSubstance,
	statLevels,
	basicDress,
	bodySlots as bodySlotsOf,
	coverSensitive,
	hidesSensitive,
	maxOf,
	remapAvatarSlots,
	unlockedBodies,
	wearOverlay,
	withoutSensitive
} from '@scutoons/content';
import type {
	Avatar,
	AvatarBody,
	AvatarBodyRef,
	AvatarSlot,
	AvatarSlotInstance,
	CharacterRegistry,
	CharacterUnlock,
	Catalog,
	Entry,
	EntryId,
	GrantSource,
	ItemDef,
	ItemVariant,
	Localized,
	ReaderState,
	Recipe
} from '@scutoons/content';
import { getLocale } from '$lib/paraglide/runtime.js';
import type { SessionContext } from './session.svelte.ts';

const INVENTORY_KEY = Symbol('inventory');
const GUEST_STORAGE_KEY = 'scutoons_inventory';
/** The guest's avatar body { character, look } (per device; the old ids sphynx / sphynx_nude / mouse are mapped) */
const GUEST_BODY_KEY = 'scutoons_avatar_body';
/** The guest's reader stats + effects (ITEMS.md "Substances"), per device */
const GUEST_READER_KEY = 'scutoons_reader';

function readGuestReader(): ReaderState {
	try {
		const raw = JSON.parse(localStorage.getItem(GUEST_READER_KEY) ?? 'null');
		return { stats: raw?.stats && typeof raw.stats === 'object' ? raw.stats : {}, effects: Array.isArray(raw?.effects) ? raw.effects : [] };
	} catch {
		return { stats: {}, effects: [] };
	}
}
const LEGACY_BODIES: Record<string, AvatarBodyRef> = {
	sphynx: { character: 'catpope', look: 'swim' },
	sphynx_nude: { character: 'catpope', look: 'naked' },
	mouse: { character: 'yellowmouse', look: 'swim' }
};

function readGuestBody(): AvatarBodyRef | null {
	try {
		const raw = localStorage.getItem(GUEST_BODY_KEY);
		if (!raw) return null;
		if (LEGACY_BODIES[raw]) return LEGACY_BODIES[raw];
		const parsed = JSON.parse(raw);
		return typeof parsed?.character === 'string' && typeof parsed?.look === 'string' ? { character: parsed.character, look: parsed.look } : null;
	} catch {
		return null;
	}
}

/** The character registry (avatar bodies and their wear slots), loaded once per page load when an avatar view needs it */
let registryPromise: Promise<CharacterRegistry | null> | null = null;
function fetchRegistryOnce(): Promise<CharacterRegistry | null> {
	registryPromise ??= fetchRegistry().catch(() => null);
	return registryPromise;
}

interface GuestState {
	entries: Entry[];
	avatar: Avatar;
	starters: string[]; // starter item ids already handed out (so crafting the camera away doesn't respawn it)
	seq: number;
}

export interface Notice {
	id: number;
	item_id: string;
	qty: number;
	/** returned = a worn item came off by a swap (back in the inventory); restored = the cover rule put it back on */
	kind: 'grant' | 'craft' | 'claim' | 'returned' | 'restored' | 'used';
	/** the inventory entry (returned / restored) */
	entry?: EntryId;
}

export type InventoryContext = ReturnType<typeof createInventoryContext>;

let _current: InventoryContext | undefined;

export function localized(value: Localized | string | null | undefined): string {
	if (!value) return '';
	if (typeof value === 'string') return value;
	return value[getLocale()] ?? value.de ?? value.en ?? Object.values(value)[0] ?? '';
}

function emptyGuest(): GuestState {
	return { entries: [], avatar: { slots: {} }, starters: [], seq: 1 };
}

/**
 * Inventory = catalog (`/items/`) + the reader's entries and avatar.
 * Logged in → server (`/me/`, `/inventory/*`, `/avatar/`); guest → localStorage `scutoons_inventory`.
 */
export function createInventoryContext(session: SessionContext) {
	let catalog = $state<Catalog | null>(null);
	let catalogError = $state<string | null>(null);
	let editionCounts = $state<Record<string, number>>({});
	let guest = $state<GuestState>(emptyGuest());
	let notices = $state<Notice[]>([]);
	let busy = $state(false);
	let noticeSeq = 0;

	const isAccount = $derived(session.isLoggedIn);
	const entries = $derived<Entry[]>(
		isAccount ? (session.me?.inventory ?? []) : guest.entries
	);
	const avatar = $derived<Avatar>(isAccount ? (session.me?.avatar ?? { slots: {} }) : guest.avatar);
	// avatar body: members from /me/ (server-checked unlocks), guests the default unlocks + a per-device choice
	let guestBody = $state<AvatarBodyRef | null>(typeof localStorage === 'undefined' ? null : readGuestBody());
	const unlocked = $derived<CharacterUnlock[]>(isAccount ? (session.me?.avatar?.unlocked ?? DEFAULT_AVATAR_UNLOCKS) : DEFAULT_AVATAR_UNLOCKS);
	const body = $derived<AvatarBodyRef | null>(isAccount ? (session.me?.avatar?.body ?? null) : guestBody);
	// the bodies the reader may pick (registry, lazily loaded) and the current one with its wear slots (slot instances)
	let registry = $state<CharacterRegistry | null>(null);
	let registryLoaded = $state(false);
	// "Hide sensitive content" (ITEMS.md "Sensitive content"): members by their setting (on by default), guests always
	const hideSensitive = $derived(hidesSensitive(isAccount ? session.me?.user : null));
	// every unlocked body incl. the sensitive (naked) looks – the wardrobe's base image (ITEMS.md "Wardrobe")
	const unlockedAll = $derived<AvatarBody[]>(unlockedBodies(avatarBodies(registry, getLocale()), unlocked));
	const bodies = $derived<AvatarBody[]>(withoutSensitive(unlockedAll, hideSensitive));
	// the chosen body; a hidden (sensitive) one shows the same character's default / first visible look instead
	const currentBody = $derived<AvatarBody | undefined>(
		bodies.find((b) => b.character === body?.character && b.look === body?.look) ??
			bodies.find((b) => b.character === body?.character && b.default) ??
			bodies.find((b) => b.character === body?.character) ??
			bodies.find((b) => b.default) ??
			bodies[0]
	);
	const slots = $derived<AvatarSlotInstance[]>(currentBody?.slots ?? bodySlotsOf(null));
	const itemsById = $derived<Record<string, ItemDef>>(
		Object.fromEntries((catalog?.items ?? []).map((i) => [i.id, i]))
	);
	const slotTypeOf = (id: EntryId): AvatarSlot | undefined => {
		const entry = entries.find((e) => String(e.id) === String(id));
		return entry ? (itemsById[entry.item_id]?.slot ?? undefined) : undefined;
	};
	/** the wearable entries in catalog order (the cover rule takes the first unworn one: the swim pants before the shorts) */
	const wearableEntries = $derived(
		entries
			.map((e) => ({ e, i: (catalog?.items ?? []).findIndex((it) => it.id === e.item_id) }))
			.filter(({ e }) => !!itemsById[e.item_id]?.slot)
			.sort((a, b) => a.i - b.i)
			.map(({ e }) => ({ id: e.id, slot: itemsById[e.item_id]?.slot ?? null }))
	);
	// reader stats + effects (ITEMS.md "Substances"): members from /me/, guests per device; `now` ticks so levels decay
	// and effects end on screen
	let guestReader = $state<ReaderState>(typeof localStorage === 'undefined' ? { stats: {}, effects: [] } : readGuestReader());
	let now = $state(Date.now());
	if (typeof window !== 'undefined') setInterval(() => (now = Date.now()), 15_000);
	const reader = $derived<ReaderState>(isAccount ? { stats: session.me?.stats ?? {}, effects: session.me?.effects ?? [] } : guestReader);
	/** every catalog stat with its current level (members: the server's decayed level from /me/, re-decayed locally) */
	const stats = $derived(statLevels(catalog, reader, now));
	const effects = $derived(activeEffects(reader.effects, now));
	/** worn items by slot instance of the current body (stored ones moved onto it: same id, else by type) */
	const worn = $derived<Record<string, EntryId | null>>(remapAvatarSlots(avatar.slots ?? {}, slots, slotTypeOf));

	/** Loads the registry (avatar bodies) once; resolves when it is there (or failed). */
	async function loadRegistry() {
		if (registryLoaded) return;
		const r = await fetchRegistryOnce();
		registry = r;
		registryLoaded = true;
	}

	// ---------- guest storage ----------

	function loadGuest() {
		try {
			const raw = localStorage.getItem(GUEST_STORAGE_KEY);
			if (raw) guest = { ...emptyGuest(), ...JSON.parse(raw) };
		} catch {
			guest = emptyGuest();
		}
	}

	function saveGuest() {
		try {
			localStorage.setItem(GUEST_STORAGE_KEY, JSON.stringify(guest));
		} catch {
			/* storage unavailable */
		}
	}

	function resetGuest() {
		guest = emptyGuest();
		giveGuestStarters();
		saveGuest();
	}

	function giveGuestStarters() {
		if (!catalog) return;
		let changed = false;
		const grantedTypes = new Set<string>();
		for (const item of catalog.items) {
			if (item.starter && !guest.starters.includes(item.id)) {
				guest.starters.push(item.id);
				addGuestEntry(item.id, 1, 'starter');
				if (item.slot) grantedTypes.add(item.slot);
				changed = true;
			}
		}
		// the basic dress (ITEMS.md "Wardrobe": T-shirt, shorts, sandals – the swim pants on a body without a shorts
		// overlay) goes on for the slot types just handed out, where nothing of that type is worn; stored by slot type,
		// `remapAvatarSlots` puts it on the body's first free instance of that type
		if (grantedTypes.size) {
			const character = guestBody?.character ?? DEFAULT_AVATAR_UNLOCKS[0]?.character ?? null;
			const wornTypes = Object.values(guest.avatar.slots).map((id) => (id == null ? null : itemsById[guest.entries.find((e) => String(e.id) === String(id))?.item_id ?? '']?.slot));
			for (const item of basicDress(catalog.items.filter((i) => i.starter && i.slot), character)) {
				if (!item.slot || !grantedTypes.has(item.slot) || wornTypes.includes(item.slot)) continue;
				const entry = guest.entries.find((e) => e.item_id === item.id);
				if (entry) guest.avatar = { ...guest.avatar, slots: { ...guest.avatar.slots, [item.slot]: entry.id } };
			}
		}
		if (changed) saveGuest();
	}

	function addGuestEntry(item_id: string, qty: number, source: Entry['source'], variant: string | null = null) {
		const item = itemsById[item_id];
		const existing = item?.stackable
			? guest.entries.find((e) => e.item_id === item_id && e.variant === variant)
			: undefined;
		if (existing) {
			existing.qty = Math.min(existing.qty + qty, maxOf(item));
			return existing;
		}
		const entry: Entry = {
			id: `local-${guest.seq++}`,
			item_id,
			variant,
			qty: item?.stackable ? qty : 1,
			source,
			code: null,
			edition_number: null,
			acquired_at: new Date().toISOString()
		};
		guest.entries.push(entry);
		// non-stackable: one entry per piece
		if (!item?.stackable) for (let i = 1; i < qty; i++) addGuestEntry(item_id, 1, source, variant);
		return entry;
	}

	function removeGuestItems(item_id: string, qty: number) {
		let left = qty;
		for (const entry of [...guest.entries]) {
			if (left <= 0) break;
			if (entry.item_id !== item_id) continue;
			const take = Math.min(entry.qty, left);
			entry.qty -= take;
			left -= take;
			if (entry.qty <= 0) {
				guest.entries = guest.entries.filter((e) => e !== entry);
				for (const slot of Object.keys(guest.avatar.slots)) {
					if (guest.avatar.slots[slot] === entry.id) guest.avatar.slots[slot] = null;
				}
			}
		}
	}

	// ---------- catalog ----------

	async function loadCatalog() {
		try {
			catalog = await fetchCatalog();
			catalogError = null;
			giveGuestStarters();
		} catch (e) {
			catalogError = e instanceof Error ? e.message : String(e);
			console.warn('Item catalog unavailable:', e);
		}
		try {
			const editions = await fetchEditions();
			editionCounts = Object.fromEntries(
				(Array.isArray(editions) ? editions : [])
					.filter((e) => e.edition_count)
					.map((e) => [e.id, e.edition_count as number])
			);
		} catch {
			/* optional */
		}
	}

	// ---------- helpers ----------

	function item(id: string): ItemDef | undefined {
		return itemsById[id];
	}

	function variantOf(entry: Pick<Entry, 'item_id' | 'variant'>): ItemVariant | undefined {
		if (!entry.variant) return undefined;
		return item(entry.item_id)?.variants?.find((v) => v.id === entry.variant);
	}

	function iconOf(entry: Pick<Entry, 'item_id' | 'variant'>): string {
		return variantOf(entry)?.icon || item(entry.item_id)?.icon || '';
	}

	function nameOf(entry: Pick<Entry, 'item_id' | 'variant'>): string {
		const def = item(entry.item_id);
		const base = def ? localized(def.name) : entry.item_id;
		const variant = variantOf(entry);
		return variant ? `${base} · ${localized(variant.name)}` : base;
	}

	function count(item_id: string): number {
		return entries.filter((e) => e.item_id === item_id).reduce((sum, e) => sum + e.qty, 0);
	}

	function editionTotal(item_id: string): number | null {
		const def = item(item_id);
		return def?.edition_id ? (editionCounts[def.edition_id] ?? null) : null;
	}

	function canCraft(recipe: Recipe): boolean {
		return recipe.inputs.every((i) => count(i.item) >= i.qty);
	}

	/** Recipes that use both items (combine mode). */
	function recipesFor(a: string, b: string): Recipe[] {
		return (catalog?.recipes ?? []).filter((r) => {
			const ids = r.inputs.map((i) => i.item);
			return a === b
				? r.inputs.some((i) => i.item === a && i.qty >= 2)
				: ids.includes(a) && ids.includes(b);
		});
	}

	/** the entry worn on a slot instance of the current body */
	function equippedEntry(slotId: string): Entry | undefined {
		const id = worn[slotId];
		return id == null ? undefined : entries.find((e) => String(e.id) === String(id));
	}

	function isEquipped(entry: Entry): boolean {
		return Object.values(worn).some((id) => id != null && String(id) === String(entry.id));
	}

	function notify(item_id: string, qty: number, kind: Notice['kind'], entry?: EntryId) {
		const notice = { id: ++noticeSeq, item_id, qty, kind, entry };
		notices = [...notices, notice];
		setTimeout(() => (notices = notices.filter((n) => n.id !== notice.id)), 3500);
	}

	function applyInventory(inventory: Entry[] | undefined) {
		if (inventory) session.patchMe({ inventory });
	}

	// ---------- mutations ----------

	/**
	 * Grant an item from a client-side source (story, scan). Silently ignored if not grantable. `error` = the API's
	 * error code when the server refused (e.g. `daily_limit`, `cooldown`), `failed` otherwise.
	 */
	async function grant(item_id: string, source: GrantSource, qty = 1, ref?: string): Promise<{ ok: boolean; error?: string }> {
		const def = item(item_id);
		if (!def || !def.grantable?.includes(source) || qty <= 0) return { ok: false, error: 'not_grantable' };
		if (count(item_id) >= maxOf(def)) return { ok: false, error: 'max_reached' };
		if (isAccount) {
			try {
				let result;
				for (let i = 0; i < Math.min(qty, 10); i++) result = await grantItem(item_id, source, ref);
				applyInventory(result?.inventory);
			} catch (e: any) {
				console.warn('Grant failed', e);
				return { ok: false, error: typeof e?.code === 'string' ? e.code : 'failed' };
			}
		} else {
			addGuestEntry(item_id, qty, source);
			saveGuest();
		}
		notify(item_id, qty, 'grant');
		return { ok: true };
	}

	async function craft(recipe: Recipe): Promise<{ ok: boolean; error?: string }> {
		if (!canCraft(recipe)) return { ok: false, error: 'missing_inputs' };
		busy = true;
		try {
			if (isAccount) {
				const result = await craftRecipe(recipe.id);
				applyInventory(result.inventory);
			} else {
				for (const input of recipe.inputs) removeGuestItems(input.item, input.qty);
				addGuestEntry(recipe.output.item, recipe.output.qty, 'craft');
				saveGuest();
			}
			notify(recipe.output.item, recipe.output.qty, 'craft');
			return { ok: true };
		} catch (e) {
			return { ok: false, error: e instanceof ApiError ? e.code : 'error' };
		} finally {
			busy = false;
		}
	}

	/** Claim (own) a QR code into the account inventory — login required. Throws ApiError (409 already_claimed |
	 * email_required, 422 choose_item | not_claimable). */
	async function claim(code: string, item_id?: string | null, variant?: string | null) {
		busy = true;
		try {
			const result = await claimCode(code, item_id, variant);
			applyInventory(result.inventory);
			await session.loadMe(); // access permissions may have changed
			const claimed = item_id ?? result.inventory?.find((e) => e.code === code)?.item_id;
			if (claimed) notify(claimed, 1, 'claim');
			return result;
		} finally {
			busy = false;
		}
	}

	/**
	 * Put `entryId` on the slot instance `slotId` (null = empty it); one entry is worn at most once. A swap: the entry
	 * that was on the instance comes off and stays in the inventory (notice "returned"). While "Hide sensitive content" is
	 * on the cover rule refills empty legs (members: the server, `restored`; guests: here) – notice "restored".
	 */
	async function setSlot(slotId: string, entryId: EntryId | null) {
		const before = { ...worn };
		const next = { ...worn };
		if (entryId != null) for (const [id, e] of Object.entries(next)) if (e != null && String(e) === String(entryId)) next[id] = null;
		next[slotId] = entryId;
		let restored: string[] = [];
		if (isAccount) {
			const result = await putAvatar({ [slotId]: entryId });
			restored = result.restored ?? [];
			session.patchMe({ avatar: result.avatar ?? { ...avatar, slots: next } });
		} else {
			const covered = hideSensitive ? coverSensitive(next, slots, wearableEntries) : { worn: next, restored: [] };
			restored = covered.restored;
			guest.avatar = { ...guest.avatar, slots: covered.worn };
			saveGuest();
		}
		announce(before, restored);
	}

	/** Notices after an avatar change: what came off (back in the inventory) and what the cover rule put back on. */
	function announce(before: Record<string, EntryId | null>, restored: string[]) {
		const now = new Set(Object.values(worn).filter((e) => e != null).map(String));
		for (const e of Object.values(before)) {
			if (e == null || now.has(String(e))) continue;
			const entry = entries.find((x) => String(x.id) === String(e));
			if (entry) notify(entry.item_id, 1, 'returned', entry.id);
		}
		// last = the newest note: what the cover rule put back on matters most
		for (const id of restored) {
			const entry = equippedEntry(id);
			if (entry) notify(entry.item_id, 1, 'restored', entry.id);
		}
	}

	/**
	 * Choose the avatar body (must be unlocked; the server checks it for members: 422 locked / no_apose). The worn items
	 * move along: same slot id, else the first free slot of their type (the server does the same for members).
	 */
	async function setBody(ref: AvatarBodyRef | null) {
		const before = { ...worn };
		let restored: string[] = [];
		if (isAccount) {
			const result = await putAvatarBody(ref);
			restored = result.restored ?? [];
			session.patchMe({ avatar: result.avatar ?? { ...avatar, body: ref } });
		} else {
			const target = ref ? bodies.find((b) => b.character === ref.character && b.look === ref.look) : (bodies.find((b) => b.default) ?? bodies[0]);
			const targetSlots = target?.slots ?? bodySlotsOf(null);
			const moved = remapAvatarSlots(worn, targetSlots, slotTypeOf);
			// guests always hide sensitive content: the new body's legs are covered too (twins: every figure)
			const covered = coverSensitive(moved, targetSlots, wearableEntries);
			restored = covered.restored;
			guest.avatar = { ...guest.avatar, slots: covered.worn };
			saveGuest();
			guestBody = ref;
			try {
				if (ref) localStorage.setItem(GUEST_BODY_KEY, JSON.stringify(ref));
				else localStorage.removeItem(GUEST_BODY_KEY);
			} catch {
				// not persisted, still switches for this visit
			}
		}
		// a body switch only announces what the cover rule put on (items that don't fit the new body just come off)
		announce({}, restored);
	}

	/** The wardrobe overlay of an entry on a slot instance of the current body (null = draw its icon at the anchor). */
	function overlayOf(entry: Pick<Entry, 'item_id'>, slotId: string): string | null {
		return wearOverlay(item(entry.item_id), currentBody?.character, slotId);
	}

	/** Every cover instance (legs) of the current body wears an item that has an overlay for it. */
	const covered = $derived(
		slots
			.filter((s) => SENSITIVE_COVER.includes(s.slot))
			.every((s) => {
				const id = worn[s.id];
				const entry = id == null ? undefined : entries.find((e) => String(e.id) === String(id));
				return !!entry && !!wearOverlay(itemsById[entry.item_id], currentBody?.character, s.id);
			})
	);

	/**
	 * Wear an entry: on `slotId` (a slot instance of the current body) or – without one – on the first free instance of
	 * the item's slot type, else the first instance of that type.
	 */
	async function equip(entry: Entry, slotId?: string) {
		const type = item(entry.item_id)?.slot;
		if (!type) return;
		await loadRegistry();
		const target = slotId ?? (slots.find((s) => s.slot === type && worn[s.id] == null) ?? slots.find((s) => s.slot === type))?.id;
		if (!target) return;
		await setSlot(target, entry.id);
	}

	/** Take off whatever is worn on a slot instance. */
	async function unequip(slotId: string) {
		await setSlot(slotId, null);
	}

	/** Take an entry off wherever it is worn. */
	async function unequipEntry(entry: Entry) {
		const slotId = Object.entries(worn).find(([, id]) => id != null && String(id) === String(entry.id))?.[0];
		if (slotId) await setSlot(slotId, null);
	}

	/**
	 * "Benutzen" on a substance: one piece is used up, its stat changes / timed effect apply (members: the server,
	 * atomically; guests: here). Game mechanics only.
	 */
	async function useSubstance(entry: Entry): Promise<{ ok: boolean; error?: string }> {
		const def = item(entry.item_id);
		if (!def || !isSubstance(def)) return { ok: false, error: 'not_usable' };
		busy = true;
		try {
			if (isAccount) {
				const result = await useItem(entry.item_id);
				session.patchMe({ inventory: result.inventory, stats: result.stats ?? {}, effects: result.effects ?? [] });
			} else {
				if (count(entry.item_id) < 1) return { ok: false, error: 'missing' };
				removeGuestItems(entry.item_id, 1);
				saveGuest();
				guestReader = applySubstance(guestReader, def, catalog?.stats ?? {}, Date.now());
				saveGuestReader();
			}
			now = Date.now();
			notify(entry.item_id, 1, 'used');
			return { ok: true };
		} catch (e) {
			return { ok: false, error: e instanceof ApiError ? e.code : 'error' };
		} finally {
			busy = false;
		}
	}

	/** "Effekt beenden" */
	async function endEffect(kind: string) {
		if (isAccount) {
			const result = await endEffectApi(kind);
			session.patchMe({ effects: result.effects ?? [] });
		} else {
			guestReader = endEffectState(guestReader, kind);
			saveGuestReader();
		}
	}

	function saveGuestReader() {
		try {
			localStorage.setItem(GUEST_READER_KEY, JSON.stringify(guestReader));
		} catch {
			/* storage unavailable */
		}
	}

	/** First login on this device: move guest items into the account (crafted items are split back into their inputs). */
	async function importGuest() {
		const totals: Record<string, number> = {};
		const add = (id: string, qty: number) => {
			const def = item(id);
			if (!def || def.starter) return;
			if (def.grantable?.length) {
				totals[id] = (totals[id] ?? 0) + qty;
				return;
			}
			const recipe = catalog?.recipes.find((r) => r.output.item === id);
			if (recipe) for (const input of recipe.inputs) add(input.item, input.qty * qty);
		};
		for (const entry of guest.entries) if (entry.source !== 'qr') add(entry.item_id, entry.qty);
		const items = Object.entries(totals).map(([item_id, qty]) => ({ item_id, qty }));
		if (items.length === 0) {
			resetGuest();
			return;
		}
		try {
			const result = await importItems(items);
			applyInventory(result.inventory);
			resetGuest();
		} catch (e) {
			console.warn('Guest inventory import failed, keeping local items', e);
		}
	}

	function init() {
		loadGuest();
		loadCatalog();
		session.subscribe((event) => {
			if (event === 'login') importGuest();
			if (event === 'logout') resetGuest();
		});
	}

	const context = {
		get catalog() {
			return catalog;
		},
		get catalogError() {
			return catalogError;
		},
		get entries() {
			return entries;
		},
		get avatar() {
			return avatar;
		},
		/** characters (+ A-pose looks) the reader may wear as avatar body */
		get unlocked() {
			return unlocked;
		},
		/** the chosen avatar body (null = the first unlocked default) */
		get body() {
			return body;
		},
		/** "Hide sensitive content" is on (guests: always) – sensitive A-pose looks are left out of `bodies` */
		get hideSensitive() {
			return hideSensitive;
		},
		/** every unlocked body incl. sensitive looks (the wardrobe's naked base) */
		get unlockedBodies() {
			return unlockedAll;
		},
		/** the current body's cover instances (legs) all wear an item with an overlay */
		get covered() {
			return covered;
		},
		overlayOf,
		/** the bodies the reader may pick (after `loadRegistry()`) */
		get bodies() {
			return bodies;
		},
		/** the body shown (the chosen one, else the first unlocked default) */
		get currentBody() {
			return currentBody;
		},
		/** the current body's wear slots (slot instances) */
		get slots() {
			return slots;
		},
		get registryLoaded() {
			return registryLoaded;
		},
		loadRegistry,
		setBody,
		get isAccount() {
			return isAccount;
		},
		get notices() {
			return notices;
		},
		get busy() {
			return busy;
		},
		/** reader stats with their current levels ("Testo 62"), ITEMS.md "Substances" */
		get stats() {
			return stats;
		},
		/** running reader effects */
		get effects() {
			return effects;
		},
		useSubstance,
		endEffect,
		init,
		loadCatalog,
		item,
		variantOf,
		iconOf,
		nameOf,
		count,
		editionTotal,
		canCraft,
		recipesFor,
		equippedEntry,
		isEquipped,
		grant,
		craft,
		claim,
		equip,
		unequip,
		unequipEntry
	};
	return context;
}

export function setInventoryContext(session: SessionContext) {
	const context = createInventoryContext(session);
	setContext(INVENTORY_KEY, context);
	_current = context;
	return context;
}

export function getInventoryContext() {
	return getContext<InventoryContext>(INVENTORY_KEY);
}

export function getCurrentInventory() {
	return _current;
}
