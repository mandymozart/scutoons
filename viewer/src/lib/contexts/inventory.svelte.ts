import { setContext, getContext } from 'svelte';
import { ApiError } from '$lib/api/client.ts';
import {
	claimCode,
	craftRecipe,
	fetchCatalog,
	fetchEditions,
	grantItem,
	importItems,
	putAvatar
} from '$lib/api/qr.service.ts';
import type {
	Avatar,
	AvatarSlot,
	Catalog,
	Entry,
	EntryId,
	GrantSource,
	ItemDef,
	ItemVariant,
	Localized,
	Recipe
} from '@scutoons/content';
import { getLocale } from '$lib/paraglide/runtime.js';
import type { SessionContext } from './session.svelte.ts';

const INVENTORY_KEY = Symbol('inventory');
const GUEST_STORAGE_KEY = 'scutoons_inventory';

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
	kind: 'grant' | 'craft' | 'claim';
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
	const itemsById = $derived<Record<string, ItemDef>>(
		Object.fromEntries((catalog?.items ?? []).map((i) => [i.id, i]))
	);

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
		for (const item of catalog.items) {
			if (item.starter && !guest.starters.includes(item.id)) {
				guest.starters.push(item.id);
				addGuestEntry(item.id, 1, 'starter');
				changed = true;
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
			existing.qty += qty;
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
				for (const slot of Object.keys(guest.avatar.slots) as AvatarSlot[]) {
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

	function equippedEntry(slot: AvatarSlot): Entry | undefined {
		const id = avatar.slots[slot];
		return id == null ? undefined : entries.find((e) => String(e.id) === String(id));
	}

	function isEquipped(entry: Entry): boolean {
		return Object.values(avatar.slots).some((id) => id != null && String(id) === String(entry.id));
	}

	function notify(item_id: string, qty: number, kind: Notice['kind']) {
		const notice = { id: ++noticeSeq, item_id, qty, kind };
		notices = [...notices, notice];
		setTimeout(() => (notices = notices.filter((n) => n.id !== notice.id)), 3500);
	}

	function applyInventory(inventory: Entry[] | undefined) {
		if (inventory) session.patchMe({ inventory });
	}

	// ---------- mutations ----------

	/** Grant an item from a client-side source (story, scan). Silently ignored if not grantable. */
	async function grant(item_id: string, source: GrantSource, qty = 1, ref?: string): Promise<boolean> {
		const def = item(item_id);
		if (!def || !def.grantable?.includes(source) || qty <= 0) return false;
		if (isAccount) {
			try {
				let result;
				for (let i = 0; i < Math.min(qty, 10); i++) result = await grantItem(item_id, source, ref);
				applyInventory(result?.inventory);
			} catch (e) {
				console.warn('Grant failed', e);
				return false;
			}
		} else {
			addGuestEntry(item_id, qty, source);
			saveGuest();
		}
		notify(item_id, qty, 'grant');
		return true;
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

	async function setSlots(slots: Avatar['slots']) {
		if (isAccount) {
			const result = await putAvatar(slots);
			session.patchMe({ avatar: result.avatar ?? { slots } });
		} else {
			guest.avatar = { slots };
			saveGuest();
		}
	}

	async function equip(entry: Entry) {
		const slot = item(entry.item_id)?.slot;
		if (!slot) return;
		await setSlots({ ...avatar.slots, [slot]: entry.id });
	}

	async function unequip(slot: AvatarSlot) {
		await setSlots({ ...avatar.slots, [slot]: null });
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
		get isAccount() {
			return isAccount;
		},
		get notices() {
			return notices;
		},
		get busy() {
			return busy;
		},
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
		unequip
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
