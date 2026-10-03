// Items, inventory, avatar — contract: agents/ITEMS.md (served by qr.scutoons.com /api)
import type { StatDef, StatEffect, TimedEffect, ReaderState } from './substances.ts';
import type { Album, MediaFormat } from './albums.ts';
import type { ItemRespawn } from './respawn.ts';

export type Localized = Record<string, string>;

export type AvatarSlot =
	| 'head'
	| 'face'
	| 'neck'
	| 'torso'
	| 'hands'
	| 'legs'
	| 'feet'
	| 'back'
	| 'accessory';

export const AVATAR_SLOTS: AvatarSlot[] = [
	'head',
	'face',
	'neck',
	'torso',
	'hands',
	'legs',
	'feet',
	'back',
	'accessory'
];

export type ItemKind = 'physical' | 'virtual';
/** `food` = eaten with the `apply` skill ("Essen" instead of "Benutzen"), e.g. the cucumber and the tuna can lower appetite */
export type ItemCategory = 'tool' | 'consumable' | 'wearable' | 'collectible' | 'media' | 'substance' | 'food';
export type GrantSource = 'story' | 'scan';
/** `gift` = given by another reader (ITEMS.md "Ownership, claims, gifting"; a code-bound gift keeps its code) */
export type EntrySource = 'qr' | 'story' | 'scan' | 'craft' | 'starter' | 'gift';

export interface ScanQrSkill {
	type: 'scan_qr';
}

export interface ScanObjectSkill {
	type: 'scan_object';
	/** ImageNet class words that count as a hit (MobileNet on the device) */
	labels: string[];
	/** the item a hit grants */
	grants: string;
	cooldown_s?: number;
	/** at most this many scan grants of `grants` per account in 24 h (server: 429 daily_limit); none = no daily limit */
	per_day?: number;
	threshold?: number;
	/** what the camera says it recognised for `labels` ("Pillendose"); none = the matched model label */
	name?: Localized;
	/**
	 * More things to find (the multi-label Check It camera, ITEMS.md "Substances"): recognised labels → the item they
	 * grant, checked together with `labels` → `grants` (the best match wins) – `scanTargets` / `matchScan`.
	 */
	map?: { labels: string[]; grants: string; name?: Localized }[];
}

export interface MinigameSkill {
	type: 'minigame';
	id: string;
}

/**
 * @deprecated (2026-10-02) the music belongs to an album now: a media item has `album` + `format` and plays only with a
 * player (`plays`), albums.ts. Still read gracefully (`legacyPlayAudio`): plays without a player.
 */
export interface PlayAudioSkill {
	type: 'play_audio';
	src: string;
}

/** "Benutzen" applies a substance ("Essen" for `food`): one piece is used up, its `effects` / `effect` hit the reader (substances.ts) */
export interface ApplySkill {
	type: 'apply';
}

export type Skill = ScanQrSkill | ScanObjectSkill | MinigameSkill | PlayAudioSkill | ApplySkill;

export interface ItemVariant {
	id: string;
	name: Localized;
	icon?: string | null;
}

export interface ItemDef {
	id: string;
	toon: string | null;
	name: Localized;
	description: Localized;
	kind: ItemKind;
	category: ItemCategory;
	icon: string;
	stackable: boolean;
	starter: boolean;
	grantable: GrantSource[];
	slot: AvatarSlot | null;
	skills: Skill[];
	edition_id: string | null;
	product: string | null;
	variants: ItemVariant[] | null;
	/**
	 * Wardrobe overlays of a wearable (ITEMS.md "Wardrobe"): `<character>` or `<character>/<slot instance id>` → CDN
	 * path of a full-body overlay aligned to that character's A-pose (768 × 1152, transparent WebP, convention
	 * `characters/<character>/wear/<item>[-<slot instance>].webp`). A body without an overlay draws the item's icon at
	 * the slot anchor instead.
	 */
	wear?: Record<string, string> | null;
	/**
	 * Part of the basic dress (ITEMS.md "Wardrobe": T-shirt, shorts, sandals): a starter wearable that is put on when it
	 * is handed out (signup, older accounts, guests). A body without an overlay for it wears another starter of the same
	 * slot that has one instead (the swim pants on yellowmouse / Top Toy) – `basicDress`.
	 */
	default_worn?: boolean;
	/**
	 * When a picked-up placement of this item is back on its slice (respawn.ts): a default interval and the range a
	 * placement may choose; none / null = it never reappears (a one-time find). Admin → Catalog can override it.
	 */
	respawn?: ItemRespawn | null;
	/** a substance's stat changes (`catalog.stats` ids), ITEMS.md "Substances" */
	effects?: StatEffect[] | null;
	/** a substance's temporary reader effect */
	effect?: TimedEffect | null;
	/**
	 * Collection limit: at most this many per account (owner 2026-10-02); none = `MAX_PER_ACCOUNT` (99). Rarer items set
	 * less. The server refuses more (409 max_reached); a claimed physical product (QR code) is always added.
	 */
	max?: number;
	/**
	 * Can a reader gift it to another reader (ITEMS.md "Ownership, claims, gifting")? None = yes, except starters / the
	 * basic dress and membership / access-only items (`isGiftable`). The server decides (403 not_giftable).
	 */
	giftable?: boolean;
	/**
	 * A media item's album (`catalog.albums` id) and its sound carrier (agents/ITEMS.md "Players, albums, related
	 * items"): it plays in the app only for a reader who owns a player of that format.
	 */
	album?: string | null;
	format?: MediaFormat | null;
	/** A player: the formats it plays (`record_player` → `['vinyl']`) */
	plays?: MediaFormat[] | null;
}

/** Can this item be given to another reader? The same rule as the server's `Catalog::giftable`. */
export function isGiftable(item: Pick<ItemDef, 'giftable' | 'starter' | 'default_worn' | 'category'> | null | undefined): boolean {
	if (!item) return false;
	if (typeof item.giftable === 'boolean') return item.giftable;
	return !item.starter && !item.default_worn && !['membership', 'access'].includes(item.category as string);
}

/** Default collection limit per item and account (owner 2026-10-02). */
export const MAX_PER_ACCOUNT = 99;

/** How many of an item one account may hold. */
export function maxOf(item: Pick<ItemDef, 'max'> | null | undefined): number {
	return Math.max(1, item?.max ?? MAX_PER_ACCOUNT);
}

export interface RecipePart {
	item: string;
	qty: number;
}

export interface Recipe {
	id: string;
	inputs: RecipePart[];
	output: RecipePart;
}

export interface Catalog {
	version: number;
	slots: AvatarSlot[];
	items: ItemDef[];
	recipes: Recipe[];
	/** the reader stats substances change (`testo`, `appetite`): baseline + half-life, ITEMS.md "Substances" */
	stats?: Record<string, StatDef>;
	/** the music media items carry (albums.ts), language-neutral */
	albums?: Album[];
}

export type EntryId = number | string;

export interface Entry {
	id: EntryId;
	item_id: string;
	variant: string | null;
	qty: number;
	source: EntrySource;
	code: string | null;
	edition_number: number | null;
	acquired_at: string;
}

/** Where an avatar-body unlock came from (server `member_characters.source`) */
export type UnlockSource = 'default' | 'code' | 'item' | 'story' | 'admin';

/** A character the member may wear as avatar body, with the unlocked A-pose looks (`/me/ avatar.unlocked`) */
export interface CharacterUnlock {
	character: string;
	/** A-pose look ids (`swim`, `naked`, …) */
	looks: string[];
	source: UnlockSource;
}

/** The chosen avatar body: a character's A-pose look */
export interface AvatarBodyRef {
	character: string;
	look: string;
}

/**
 * The default unlocks of every member and of guests (owner 2026-09-30: Cat Pope, swim + naked). The server's value is
 * the setting AVATAR_DEFAULT_UNLOCKS (members get theirs from `/me/`); guests use this one.
 */
export const DEFAULT_AVATAR_UNLOCKS: CharacterUnlock[] = [{ character: 'catpope', looks: ['swim', 'naked'], source: 'default' }];

export interface Avatar {
	/**
	 * worn items by slot instance id of the current body (`bodySlots` of its A-pose look: `torso`, `torso-left`, …;
	 * members: every instance of the body, null = empty) – ITEMS.md "Slots per avatar body"
	 */
	slots: Record<string, EntryId | null>;
	/** the chosen body (null = the first unlocked default) – ITEMS.md "Reader avatar bodies & character unlocks" */
	body?: AvatarBodyRef | null;
	/** what the member has unlocked (defaults first) */
	unlocked?: CharacterUnlock[];
}

export interface User {
	id: number | string;
	email: string;
	created_at: string;
	/** account roles from `/me/` (`member`, `creator`, `admin`) – ITEMS.md "Roles" */
	roles?: string[];
	/** "Send me news about new releases" (unchecked at signup, toggle on the account page) */
	marketing_opt_in?: boolean;
	/** "Hide sensitive content" (on by default; `PUT /me/ { hide_sensitive }`) – sensitive.ts, ITEMS.md "Sensitive content" */
	hide_sensitive?: boolean;
	/** "Media quality" (`auto` by default; `PUT /me/ { media_quality }`) – media-quality.ts */
	media_quality?: 'auto' | 'low' | 'medium' | 'high';
}

/** `/me/ access`: permissions granted by the account's owned, non-expired codes (server permissions map) */
export interface Access {
	permissions: string[];
	access_token?: string;
	expires_at: string | null;
	codes: string[];
	/** the (promo) code the account was created with, when it grants the access – not owned */
	signup_code?: string | null;
}

export interface ServerMembership {
	code: string;
	type: string;
	expires_at: string | null;
}

export interface Me {
	user: User;
	memberships: ServerMembership[];
	access?: Access;
	/** server-signed access token for the account's permissions */
	access_token?: string;
	inventory: Entry[];
	avatar: Avatar;
	/** stored stat levels (decay with `statLevel`) and running effects – ITEMS.md "Substances" */
	stats?: ReaderState['stats'];
	effects?: ReaderState['effects'];
	/** what arrived and was not seen yet: gifts and codes an admin moved here (`POST /inventory/received/seen/`) */
	received?: ReceivedNotice[];
}

/**
 * "X schenkt dir Y" (ITEMS.md "Ownership, claims, gifting"): a gift (`from` = the giver's email) or a code that an admin
 * moved into this account after a claim (`kind: claim | admin`, no previous owner shown)
 */
export interface ReceivedNotice {
	id: number;
	kind: 'gift' | 'claim' | 'admin';
	item_id: string | null;
	variant: string | null;
	qty: number;
	code: string | null;
	from: string | null;
	created_at: string;
}

/** An ownership claim as the claimant sees it ("This is mine", `GET /claims/`) – never who owns the code */
export interface OwnershipClaim {
	id: string;
	code: string;
	status: 'open' | 'transferred' | 'rejected' | 'withdrawn';
	statement: string;
	files: { id: string; name: string; type: string; size: number }[];
	created_at: string;
	decided_at: string | null;
}

export interface CodeItem {
	item_id: string;
	variant: string | null;
	qty: number;
}

export interface CodeOption {
	item_id: string;
	variants: (string | ItemVariant)[] | null;
}

export interface CodeInfo {
	code: string;
	type: string;
	/** can be owned by one account (membership); promo codes are not */
	claimable?: boolean;
	/** a new account can be created with this code (membership, promo) */
	signup?: boolean;
	claimed: boolean;
	claimed_by_me?: boolean;
	items: CodeItem[];
	options: CodeOption[];
	edition: { id: string; name: string; number?: number | null; count?: number | null } | null;
}

export interface Edition {
	id: string;
	name: string;
	edition_count?: number;
}
