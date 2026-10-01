// Items, inventory, avatar — contract: agents/ITEMS.md (served by qr.scutoons.com /api)
import type { StatDef, StatEffect, TimedEffect, ReaderState } from './substances.ts';

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
export type ItemCategory = 'tool' | 'consumable' | 'wearable' | 'collectible' | 'media' | 'substance';
export type GrantSource = 'story' | 'scan';
export type EntrySource = 'qr' | 'story' | 'scan' | 'craft' | 'starter';

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

export interface PlayAudioSkill {
	type: 'play_audio';
	src: string;
}

/** "Benutzen" applies a substance: one piece is used up, its `effects` / `effect` hit the reader (substances.ts) */
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
	/** a substance's stat changes (`catalog.stats` ids), ITEMS.md "Substances" */
	effects?: StatEffect[] | null;
	/** a substance's temporary reader effect */
	effect?: TimedEffect | null;
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
