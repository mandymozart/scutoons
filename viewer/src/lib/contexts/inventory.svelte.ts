import { setContext, getContext } from 'svelte';
import { ApiError } from '$lib/api.ts';
import {
	claimCode,
	craftRecipe,
	moveEntry,
	placeFurniture,
	payRide,
	fetchCatalog,
	fetchEditions,
	grantItem,
	takeItem,
	importItems,
	putAvatar,
	putAvatarBody,
	fetchRegistry,
	useItem,
	endEffect as endEffectApi,
	giftItem,
	markReceivedSeen,
	fetchInvites,
	createInvite,
	recallInvite as recallInviteApi,
	inviteAddress
} from '$lib/api.ts';
import type { InviteOffer, InvitesResult } from '@scutoons/api';
import {
	breaksAt,
	fitsIn,
	moveInCheck,
	DEFAULT_AVATAR_UNLOCKS,
	DEFAULT_LOOK,
	REGULAR_PERSONA,
	activeEffects,
	applySubstance,
	avatarBodies,
	characterImage,
	endEffect as endEffectState,
	isSubstance,
	statLevels,
	basicDress,
	bodySlots as bodySlotsOf,
	maxOf,
	remapAvatarSlots,
	unlockedBodies
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
	EntryHome,
	EntryId,
	GrantSource,
	ItemDef,
	ReceivedNotice,
	ItemVariant,
	Localized,
	ReaderState,
	Recipe,
	RecurrenceInterval
} from '@scutoons/content';
import { getLocale } from '$lib/paraglide/runtime.js';
import type { SessionContext } from './session.svelte.ts';
import { showChrome } from '$lib/chrome.ts';

const INVENTORY_KEY = Symbol('inventory');
const GUEST_STORAGE_KEY = 'scutoons_inventory';
/**
 * The guest's avatar body { character, persona, look } (per device). Older values are read once and rewritten
 * (RULES.md: data in readers' browsers): the ids sphynx / sphynx_nude / mouse and { character, look: 'swim' } (the
 * character-level A-pose before 2026-10-13) → the character's regular default look, where the A-pose moved to.
 */
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
const LEGACY_BODIES: Record<string, string> = { sphynx: 'catpope', sphynx_nude: 'catpope', mouse: 'yellowmouse' };

function readGuestBody(): AvatarBodyRef | null {
	try {
		const raw = localStorage.getItem(GUEST_BODY_KEY);
		if (!raw) return null;
		const parsed = LEGACY_BODIES[raw] ? { character: LEGACY_BODIES[raw] } : JSON.parse(raw);
		if (typeof parsed?.character !== 'string') return null;
		if (typeof parsed.persona === 'string' && typeof parsed.look === 'string') return { character: parsed.character, persona: parsed.persona, look: parsed.look };
		const body = { character: parsed.character, persona: REGULAR_PERSONA, look: DEFAULT_LOOK };
		localStorage.setItem(GUEST_BODY_KEY, JSON.stringify(body));
		return body;
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
	/** returned = a worn item came off by a swap (back in the inventory) */
	kind: 'grant' | 'taken' | 'craft' | 'claim' | 'returned' | 'used' | 'gifted' | 'gift_received' | 'now_yours' | 'refused' | 'place' | 'invited' | 'recalled' | 'message';
	/** message: a plain text (the home: "This avatar has no home yet.") */
	text?: string;
	/** place: a map place was unlocked (agents/MAP.md §3.4, §8.5a) – its name and image (the toast's icon) */
	place?: { name: string; image?: string; id?: string };
	/** refused: why the item could not be taken (the API's error code, or max_reached / not_grantable / failed …) */
	reason?: string;
	/** the inventory entry (returned) */
	entry?: EntryId;
	/**
	 * gifted / invited / recalled: the recipient's email · gift_received: the giver's email (ITEMS.md "Ownership,
	 * claims, gifting"; invites: research/GIFT-INVITES.md)
	 */
	email?: string | null;
	/** the same notice again while it shows: counted here ("× 2") instead of one more toast (owner 2026-10-08) */
	repeat?: number;
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
	// the unlocked bodies: every unlocked look with an A-pose (ITEMS.md "Reader avatar bodies")
	const bodies = $derived<AvatarBody[]>(unlockedBodies(avatarBodies(registry, getLocale()), unlocked));
	// the chosen body; an unknown look (e.g. one that lost its A-pose) shows the same character's default body instead
	const currentBody = $derived<AvatarBody | undefined>(
		bodies.find((b) => b.character === body?.character && b.persona === body?.persona && b.look === body?.look) ??
			bodies.find((b) => b.character === body?.character && b.default) ??
			bodies.find((b) => b.character === body?.character) ??
			bodies.find((b) => b.default) ??
			bodies[0]
	);
	const slots = $derived<AvatarSlotInstance[]>(currentBody?.slots ?? bodySlotsOf(null));
	/**
	 * the current body's face for markers (owner 2026-10-14: the map shows "a real avatar picture, not the A-pose"): the
	 * look's avatar image, else its portrait, else the A-pose
	 */
	const currentAvatar = $derived.by<string>(() => {
		const b = currentBody;
		if (!b) return '';
		const character = registry?.characters?.find((c) => c.id === b.character);
		const ref = { persona: b.persona, look: b.look };
		const missing = (path: string) => !path || path.includes('/_missing/');
		for (const view of ['avatar', 'portrait']) {
			const path = characterImage(character, ref, view, { substitute: '' });
			if (!missing(path)) return path;
		}
		return b.path;
	});
	const itemsById = $derived<Record<string, ItemDef>>(
		Object.fromEntries((catalog?.items ?? []).map((i) => [i.id, i]))
	);
	const slotTypeOf = (id: EntryId): AvatarSlot | undefined => {
		const entry = entries.find((e) => String(e.id) === String(id));
		return entry ? (itemsById[entry.item_id]?.slot ?? undefined) : undefined;
	};
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

	// ---------- containers (research/SCORES.md §2: items that hold items, moved by hand) ----------

	const sameId = (a: EntryId | null | undefined, b: EntryId | null | undefined) => a != null && b != null && String(a) === String(b);
	/** the container an entry lies in */
	function containerOf(entry: Pick<Entry, 'in'>): Entry | undefined {
		return entry.in != null ? entries.find((c) => sameId(c.id, entry.in)) : undefined;
	}
	/** what lies in a container */
	function contentsOf(container: Pick<Entry, 'id'>): Entry[] {
		return entries.filter((e) => sameId(e.in, container.id));
	}
	const countIn = (container: Pick<Entry, 'id'>) => contentsOf(container).reduce((n, e) => n + e.qty, 0);
	/** the reader's containers this entry fits into (its rule), the one it lies in excluded */
	function containersFor(entry: Entry): Entry[] {
		const def = itemsById[entry.item_id];
		return entries.filter((c) => !sameId(c.id, entry.id) && !sameId(c.id, entry.in) && fitsIn(itemsById[c.item_id], def));
	}

	/** a guest's container past its overfill days breaks now (the server does the same on every read) */
	function settleGuest() {
		const now = Date.now();
		let changed = false;
		for (const c of guest.entries) {
			const holds = itemsById[c.item_id]?.holds;
			const at = c.overfilled_at ? Date.parse(c.overfilled_at) : null;
			const end = holds ? breaksAt({ holds }, guest.entries.filter((e) => sameId(e.in, c.id)).reduce((n, e) => n + e.qty, 0), at) : null;
			if (end == null || end > now || !holds?.overfill) continue;
			for (const e of guest.entries) if (sameId(e.in, c.id)) e.in = null;
			c.item_id = holds.overfill.breaks_into;
			c.aged_since ??= c.acquired_at;
			c.overfilled_at = null;
			changed = true;
		}
		if (changed) saveGuest();
	}

	/** sets / clears a guest container's overfill time from what it holds now */
	function recountGuest(id: EntryId) {
		const c = guest.entries.find((e) => sameId(e.id, id));
		const max = c ? itemsById[c.item_id]?.holds?.max : undefined;
		if (!c || max == null) return;
		const over = guest.entries.filter((e) => sameId(e.in, id)).reduce((n, e) => n + e.qty, 0) > max;
		if (over && !c.overfilled_at) c.overfilled_at = new Date().toISOString();
		if (!over) c.overfilled_at = null;
	}

	/**
	 * Puts an entry into a container, or takes it out (`into` null). `overfill`: the reader agreed to the warning.
	 * Errors: does_not_fit, container_in_container, worn, container_full { max }, overfill_warning { max, days }.
	 */
	async function move(entry: Entry, into: Entry | null, overfill = false): Promise<{ ok: boolean; error?: string; max?: number; days?: number }> {
		if (isAccount) {
			try {
				applyInventory((await moveEntry(entry.id, into?.id ?? null, overfill)).inventory);
				return { ok: true };
			} catch (e) {
				return e instanceof ApiError ? { ok: false, error: e.code, max: e.body?.max, days: e.body?.days } : { ok: false, error: 'failed' };
			}
		}
		const mine = guest.entries.find((e) => sameId(e.id, entry.id));
		if (!mine) return { ok: false, error: 'unknown_entry' };
		const from = mine.in ?? null;
		if (into) {
			const holds = itemsById[into.item_id]?.holds;
			if (!holds) return { ok: false, error: 'not_a_container' };
			if (itemsById[mine.item_id]?.holds) return { ok: false, error: 'container_in_container' };
			if (!fitsIn(itemsById[into.item_id], itemsById[mine.item_id])) return { ok: false, error: 'does_not_fit' };
			if (isEquipped(mine)) return { ok: false, error: 'worn' };
			const check = moveInCheck({ holds }, countIn(into) + mine.qty - 1);
			if (check === 'full') return { ok: false, error: 'container_full', max: holds.overfill?.max ?? holds.max };
			if (check === 'overfill' && !overfill) return { ok: false, error: 'overfill_warning', max: holds.max, days: holds.overfill?.days };
			mine.in = into.id;
			recountGuest(into.id);
		} else mine.in = null;
		if (from != null) recountGuest(from);
		saveGuest();
		return { ok: true };
	}

	/**
	 * Stands a furniture entry in a home or moves it inside its home (research/CAVE.md D4): once placed it stays in that
	 * home. Errors: not_furniture, furniture_stays (another home), in_container, worn.
	 */
	async function place(entry: Entry, home: EntryHome): Promise<{ ok: boolean; error?: string }> {
		if (isAccount) {
			try {
				applyInventory((await placeFurniture(entry.id, home.id, home.x, home.y, !!home.flip)).inventory);
				return { ok: true };
			} catch (e) {
				return { ok: false, error: e instanceof ApiError ? e.code : 'failed' };
			}
		}
		const mine = guest.entries.find((e) => sameId(e.id, entry.id));
		if (!mine) return { ok: false, error: 'unknown_entry' };
		if (!itemsById[mine.item_id]?.furniture) return { ok: false, error: 'not_furniture' };
		if (mine.home && mine.home.id !== home.id) return { ok: false, error: 'furniture_stays' };
		if (mine.in != null) return { ok: false, error: 'in_container' };
		if (isEquipped(mine)) return { ok: false, error: 'worn' };
		mine.home = { id: home.id, x: home.x, y: home.y, flip: !!home.flip };
		saveGuest();
		return { ok: true };
	}

	/**
	 * The fare of a ride (research/CAVE.md D8): members pay on the server, guests from the device inventory. Error:
	 * cannot_afford.
	 */
	async function payFare(network: string, cucumbers: number): Promise<{ ok: boolean; error?: string }> {
		if (isAccount) {
			try {
				applyInventory((await payRide(network)).inventory);
				return { ok: true };
			} catch (e) {
				return { ok: false, error: e instanceof ApiError ? e.code : 'failed' };
			}
		}
		if (guest.entries.filter((e) => e.item_id === 'cucumber' && !e.home).reduce((n, e) => n + e.qty, 0) < cucumbers) return { ok: false, error: 'cannot_afford' };
		removeGuestItems('cucumber', cucumbers);
		saveGuest();
		return { ok: true };
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

	/** the title lettered on a score's cover – in the score's own language (research/SCORES.md §1) */
	function letteringOf(entry: Pick<Entry, 'item_id'>): string | null {
		const def = itemsById[entry.item_id];
		return def?.score ? (def.name?.[def.score.language] ?? localized(def.name)) : null;
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

	/** the pieces the reader carries – not furniture standing in a home (the server's collection-limit count) */
	function carried(item_id: string): number {
		return entries.filter((e) => e.item_id === item_id && !e.home).reduce((sum, e) => sum + e.qty, 0);
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

	/** the tint of something worn (catalog `worn_tint`, e.g. sunglasses): the first one found, none = null */
	const wornTint = $derived.by(() => {
		for (const id of Object.values(worn)) {
			if (id == null) continue;
			const entry = entries.find((e) => String(e.id) === String(id));
			// a variant's own tint (a colour of the sunglasses), else the item's
			const tint = entry ? (variantOf(entry)?.worn_tint ?? item(entry.item_id)?.worn_tint) : null;
			if (tint) return tint;
		}
		return null;
	});

	function isEquipped(entry: Entry): boolean {
		return Object.values(worn).some((id) => id != null && String(id) === String(entry.id));
	}

	const noticeTimers = new Map<number, ReturnType<typeof setTimeout>>();
	/** "<place> unlocked" (agents/MAP.md §8.5a): the place's image as the icon, 4 s */
	/** a place was unlocked (its toast); `place` = its id – a tap on the toast plays its reveal (map-reveal.ts) */
	/** a plain toast (4 s) */
	function notifyMessage(text: string) {
		const id = ++noticeSeq;
		notices = [...notices, { id, item_id: '', qty: 0, kind: 'message', text }];
		noticeTimers.set(id, setTimeout(() => ((notices = notices.filter((n) => n.id !== id)), noticeTimers.delete(id)), 4000));
	}
	function notifyPlace(name: string, image?: string, place?: string) {
		const id = ++noticeSeq;
		notices = [...notices, { id, item_id: '', qty: 1, kind: 'place', place: { name, image, id: place } }];
		noticeTimers.set(id, setTimeout(() => ((notices = notices.filter((n) => n.id !== id)), noticeTimers.delete(id)), 4000));
	}
	function notify(item_id: string, qty: number, kind: Notice['kind'], entry?: EntryId, email?: string | null, reason?: string) {
		// an item came or went: the bag (top row) shows again (lib/chrome.ts)
		void showChrome();
		// a received gift stays a little longer ("X schenkt dir Y")
		const ms = kind === 'gift_received' || kind === 'now_yours' ? 7000 : 3500;
		const hide = (id: number) => {
			clearTimeout(noticeTimers.get(id));
			noticeTimers.set(id, setTimeout(() => ((notices = notices.filter((n) => n.id !== id)), noticeTimers.delete(id)), ms));
		};
		// the same text again while it shows (a blocked item tapped again and again): "× 2", "× 3" on it, not one more toast
		const same = notices.find((n) => n.kind === kind && n.item_id === item_id && n.qty === qty && n.reason === reason && (n.email ?? null) === (email ?? null));
		if (same) {
			notices = notices.map((n) => (n.id === same.id ? { ...n, repeat: (n.repeat ?? 1) + 1 } : n));
			return hide(same.id);
		}
		const notice = { id: ++noticeSeq, item_id, qty, kind, entry, email, reason };
		notices = [...notices, notice];
		hide(notice.id);
	}

	/**
	 * Gives an item to another reader (members only; ITEMS.md "Ownership, claims, gifting"). A code-bound piece moves
	 * its code. `error` = the API's error code (recipient_not_found, worn, cooldown, max_reached, daily_limit, …);
	 * recipient_not_found while a gift invite campaign runs for the member brings `invite` – the offer to invite them.
	 */
	async function gift(entry: Entry, to: string, qty = 1): Promise<{ ok: boolean; error?: string; invite?: InviteOffer }> {
		if (!isAccount) return { ok: false, error: 'unauthorized' };
		busy = true;
		try {
			const result = await giftItem({ entry_id: entry.id, qty, to: to.trim() });
			applyInventory(result.inventory);
			notify(entry.item_id, result.gift.qty, 'gifted', undefined, result.gift.to);
			if (entry.code) await session.loadMe(); // the code (and its access) left the account
			return { ok: true };
		} catch (e) {
			if (e instanceof ApiError) return { ok: false, error: e.code, invite: (e.body?.invite as InviteOffer | undefined) ?? undefined };
			return { ok: false, error: 'failed' };
		} finally {
			busy = false;
		}
	}

	// ── gift invites (agents/research/GIFT-INVITES.md): someone without an account gets an invite with the gift ──
	let invites = $state<InvitesResult | null>(null);
	/** the running campaign, the member's invites, gifts waiting for a shipping address (members only) */
	async function loadInvites(): Promise<void> {
		if (!isAccount) {
			invites = null;
			return;
		}
		try {
			invites = await fetchInvites();
		} catch (e) {
			console.warn('Could not load the invites', e);
		}
	}
	/**
	 * Invites someone without an account: with the member's own item (`entry`, the gift form) or the campaign's item
	 * (no entry). `error` = the API's code (has_account, already_invited, campaign_limit, sold_out, no_campaign, …).
	 */
	async function invite(to: string, entry?: Entry, qty = 1): Promise<{ ok: boolean; error?: string; status?: string }> {
		if (!isAccount) return { ok: false, error: 'unauthorized' };
		busy = true;
		try {
			const result = await createInvite({ to: to.trim(), ...(entry ? { entry_id: entry.id, qty } : {}) });
			applyInventory(result.inventory);
			// a gift the member pays waits for the payment: the notice says so (reason payment)
			notify(result.invite.item_id, result.invite.qty, 'invited', undefined, result.invite.email, result.invite.status === 'payment' ? 'payment' : undefined);
			await loadInvites();
			return { ok: true, status: result.invite.status };
		} catch (e) {
			return { ok: false, error: e instanceof ApiError ? e.code : 'failed' };
		} finally {
			busy = false;
		}
	}
	/** calls an invite back while it is not accepted – the member's own item comes back */
	async function recallInvite(id: string): Promise<{ ok: boolean; error?: string }> {
		busy = true;
		try {
			const result = await recallInviteApi(id);
			applyInventory(result.inventory);
			notify(result.invite.item_id, result.invite.qty, 'recalled', undefined, result.invite.email);
			await loadInvites();
			return { ok: true };
		} catch (e) {
			await loadInvites();
			return { ok: false, error: e instanceof ApiError ? e.code : 'failed' };
		} finally {
			busy = false;
		}
	}
	/** the shipping address of a physical gift the member joined by */
	async function setInviteAddress(id: string, address: { name: string; address: string; country: string }): Promise<{ ok: boolean; error?: string }> {
		try {
			invites = await inviteAddress(id, address);
			return { ok: true };
		} catch (e) {
			return { ok: false, error: e instanceof ApiError ? e.code : 'failed' };
		}
	}

	/** received notices already shown in this tab (the next /me/ may still list them until `seen` is stored) */
	const announced = new Set<number>();
	/**
	 * Shows the `received` notices of `/me/` ("X schenkt dir Y", a code moved here after a claim) once and tells the
	 * server they were seen. Called by the items host whenever /me/ changes.
	 */
	async function announceReceived(received: ReceivedNotice[] | undefined) {
		const fresh = (received ?? []).filter((r) => !announced.has(r.id));
		if (!fresh.length) return;
		for (const r of fresh) {
			announced.add(r.id);
			if (r.item_id) notify(r.item_id, r.qty, r.kind === 'gift' ? 'gift_received' : 'now_yours', undefined, r.from);
		}
		try {
			await markReceivedSeen(fresh.map((r) => r.id));
		} catch (e) {
			console.warn('Could not mark the received notices as seen', e);
		}
	}

	function applyInventory(inventory: Entry[] | undefined) {
		if (inventory) session.patchMe({ inventory });
	}

	// ---------- mutations ----------

	/**
	 * Grant an item from a client-side source (story, scan). Silently ignored if not grantable. `error` = the API's
	 * error code when the server refused (e.g. `daily_limit`, `cooldown`), `failed` otherwise. `granted` = pieces the
	 * server added (members; 0 = this `ref` was granted before) – with `quietWhenNone` there is no "+1" notice then.
	 */
	async function grant(
		item_id: string,
		source: GrantSource,
		qty = 1,
		ref?: string,
		options: { quietWhenNone?: boolean; respawn?: { respawn: RecurrenceInterval; visit?: string }; variant?: string | null } = {}
	): Promise<{ ok: boolean; error?: string; granted?: number }> {
		const def = item(item_id);
		if (!def || !def.grantable?.includes(source) || qty <= 0) return { ok: false, error: 'not_grantable' };
		if (carried(item_id) >= maxOf(def)) return { ok: false, error: 'max_reached' };
		let granted = qty;
		if (isAccount) {
			try {
				let result;
				granted = 0;
				for (let i = 0; i < Math.min(qty, 10); i++) {
					result = await grantItem(item_id, source, ref, options.respawn, options.variant);
					granted += Number(result?.granted ?? 1);
				}
				applyInventory(result?.inventory);
			} catch (e: any) {
				console.warn('Grant failed', e);
				return { ok: false, error: typeof e?.code === 'string' ? e.code : 'failed' };
			}
		} else {
			addGuestEntry(item_id, qty, source, options.variant && def.variants?.some((v) => v.id === options.variant) ? options.variant : null);
			saveGuest();
		}
		if (granted > 0 || !options.quietWhenNone) notify(item_id, granted > 0 ? granted : qty, 'grant');
		return { ok: true, granted };
	}

	/**
	 * A story takes an item (VN takeItem / a choice's take – BACKLOG 10.38): all or nothing, decided here at once from
	 * what the reader has (a trade's next actions depend on it); only a `takeable` item, never a physical one. Members:
	 * the server takes too (`ref` = once per ref) and its answer updates the inventory.
	 */
	function take(item_id: string, qty: number, ref?: string): boolean {
		const def = item(item_id);
		if (!def || def.kind === 'physical' || def.takeable !== true || qty <= 0 || count(item_id) < qty) return false;
		if (isAccount) {
			void takeItem(item_id, qty, ref)
				.then((result) => applyInventory(result.inventory))
				.catch((e) => console.warn('Take failed', e));
		} else {
			removeGuestItems(item_id, qty);
			saveGuest();
		}
		notify(item_id, qty, 'taken');
		return true;
	}

	/** Tells the reader an item could not be taken (a placed item's pickup) and why (`reason` = an error code). */
	function refused(item_id: string, reason: string) {
		notify(item_id, 0, 'refused', undefined, null, reason);
	}

	/**
	 * `use` = which entry of an input (the combine mode's two picks: the plastic's colour, the book). The same rules as
	 * the server (research/SCORES.md §2a): a `keep` input stays (the drill), the output takes `variant_from`'s variant,
	 * a used-up container hands its contents and its age to the output.
	 */
	/** `workshop`: where a recipe `at: "workshop"` is made – the reader's home or a friend's (visit link; research/CAVE.md D4) */
	async function craft(recipe: Recipe, use: Record<string, EntryId> = {}, workshop?: { home: string } | { visit: string }): Promise<{ ok: boolean; error?: string }> {
		if (!canCraft(recipe)) return { ok: false, error: 'missing_inputs' };
		busy = true;
		try {
			if (isAccount) {
				const result = await craftRecipe(recipe.id, use, workshop);
				applyInventory(result.inventory);
			} else {
				let variant: string | null = null;
				let carried: Entry[] = [];
				let agedSince: string | null = null;
				for (const input of recipe.inputs) {
					if (input.keep) continue;
					const container = !!itemsById[input.item]?.holds;
					const picked = input.item === recipe.output.variant_from || container
						? (guest.entries.find((e) => sameId(e.id, use[input.item]) && e.item_id === input.item) ?? guest.entries.find((e) => e.item_id === input.item))
						: undefined;
					let left = input.qty;
					if (picked) {
						if (input.item === recipe.output.variant_from) variant = picked.variant ?? null;
						if (container) {
							carried = [...carried, ...guest.entries.filter((e) => sameId(e.in, picked.id))];
							agedSince ??= picked.aged_since ?? picked.acquired_at;
						}
						const take = Math.min(left, picked.qty);
						picked.qty -= take;
						left -= take;
						if (picked.qty <= 0) guest.entries = guest.entries.filter((e) => e !== picked);
					}
					if (left > 0) removeGuestItems(input.item, left);
				}
				const output = itemsById[recipe.output.item];
				const made = addGuestEntry(recipe.output.item, recipe.output.qty, 'craft', output?.variants?.some((v) => v.id === variant) ? variant : null);
				if (agedSince && output?.ages) made.aged_since = agedSince;
				for (const e of carried) e.in = output?.holds && fitsIn(output, itemsById[e.item_id]) ? made.id : null;
				if (output?.holds) recountGuest(made.id);
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
	 * that was on the instance comes off and stays in the inventory (notice "returned"). Every slot may be empty.
	 */
	async function setSlot(slotId: string, entryId: EntryId | null) {
		const before = { ...worn };
		const next = { ...worn };
		if (entryId != null) for (const [id, e] of Object.entries(next)) if (e != null && String(e) === String(entryId)) next[id] = null;
		next[slotId] = entryId;
		if (isAccount) {
			const result = await putAvatar({ [slotId]: entryId });
			session.patchMe({ avatar: result.avatar ?? { ...avatar, slots: next } });
		} else {
			guest.avatar = { ...guest.avatar, slots: next };
			saveGuest();
		}
		announce(before);
	}

	/** Notices after an avatar change: what came off (back in the inventory). */
	function announce(before: Record<string, EntryId | null>) {
		const now = new Set(Object.values(worn).filter((e) => e != null).map(String));
		for (const e of Object.values(before)) {
			if (e == null || now.has(String(e))) continue;
			const entry = entries.find((x) => String(x.id) === String(e));
			if (entry) notify(entry.item_id, 1, 'returned', entry.id);
		}
	}

	/**
	 * Choose the avatar body (must be unlocked; the server checks it for members: 422 locked / no_apose). The worn items
	 * move along: same slot id, else the first free slot of their type (the server does the same for members).
	 */
	async function setBody(ref: AvatarBodyRef | null) {
		if (isAccount) {
			const result = await putAvatarBody(ref);
			session.patchMe({ avatar: result.avatar ?? { ...avatar, body: ref } });
		} else {
			const target = ref ? bodies.find((b) => b.character === ref.character && b.persona === ref.persona && b.look === ref.look) : (bodies.find((b) => b.default) ?? bodies[0]);
			const targetSlots = target?.slots ?? bodySlotsOf(null);
			guest.avatar = { ...guest.avatar, slots: remapAvatarSlots(worn, targetSlots, slotTypeOf) };
			saveGuest();
			guestBody = ref;
			try {
				if (ref) localStorage.setItem(GUEST_BODY_KEY, JSON.stringify(ref));
				else localStorage.removeItem(GUEST_BODY_KEY);
			} catch {
				// not persisted, still switches for this visit
			}
		}
		// items that don't fit the new body just come off (no notice)
	}

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
		// per item and variant (a guest's orange and blue sunglasses stay two things)
		const totals = new Map<string, { item_id: string; qty: number; variant: string | null }>();
		const add = (id: string, qty: number, variant: string | null = null) => {
			const def = item(id);
			if (!def || def.starter) return;
			if (def.grantable?.length) {
				const key = `${id}|${variant ?? ''}`;
				const t = totals.get(key) ?? { item_id: id, qty: 0, variant };
				t.qty += qty;
				totals.set(key, t);
				return;
			}
			const recipe = catalog?.recipes.find((r) => r.output.item === id);
			if (recipe) for (const input of recipe.inputs) add(input.item, input.qty * qty);
		};
		// a container and what lies in it go one by one with their local ids, so the score lies in the book again
		const linked: { item_id: string; qty: number; variant: string | null; local_id: string; in?: string }[] = [];
		for (const entry of guest.entries) {
			if (entry.source === 'qr') continue;
			const def = item(entry.item_id);
			if ((def?.holds || entry.in != null) && def?.grantable?.length && !def.starter) {
				linked.push({ item_id: entry.item_id, qty: entry.qty, variant: entry.variant ?? null, local_id: String(entry.id), ...(entry.in != null ? { in: String(entry.in) } : {}) });
			} else add(entry.item_id, entry.qty, entry.variant ?? null);
		}
		const items = [...totals.values(), ...linked];
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
		void loadCatalog().then(() => !isAccount && settleGuest());
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
		get wornTint() {
			return wornTint;
		},
		get entries() {
			return entries;
		},
		get avatar() {
			return avatar;
		},
		/** characters (+ looks with an A-pose) the reader may wear as avatar body */
		get unlocked() {
			return unlocked;
		},
		/** the chosen avatar body (null = the first unlocked default) */
		get body() {
			return body;
		},
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
		/** the current body's avatar picture (its portrait, else its A-pose) – markers such as the map's avatar */
		get currentAvatar() {
			return currentAvatar;
		},
		/** the character registry (avatar bodies, homes) once loaded */
		get registry() {
			return registry;
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
		notifyPlace,
		notifyMessage,
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
		letteringOf,
		nameOf,
		count,
		carried,
		editionTotal,
		canCraft,
		recipesFor,
		equippedEntry,
		isEquipped,
		grant,
		take,
		refused,
		craft,
		claim,
		gift,
		containerOf,
		contentsOf,
		containersFor,
		move,
		place,
		payFare,
		get invites() {
			return invites;
		},
		loadInvites,
		invite,
		recallInvite,
		setInviteAddress,
		announceReceived,
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
