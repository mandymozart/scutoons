// Items, inventory, avatar — contract: agents/ITEMS.md (served by qr.scutoons.com /api)
import type { StatDef, StatEffect, TimedEffect, ReaderState } from './substances.ts';
import type { Album, MediaFormat } from './albums.ts';
import type { ItemRespawn } from './respawn.ts';
import type { FxSoundChoice } from './fx.ts';
import type { AcceptedWarnings } from './accepted.ts';
import type { ItemAges, ItemHolds, ScoreFiles } from './containers.ts';
import type { AirliftCarrier, EntryHome, HomeScene, ItemFurniture } from './homes.ts';

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

/**
 * The slot types of the avatar model (code, not catalog data – owner 2026-10-04: "slots are merely enums for an avatar
 * model"); the server serves the same as `slots` in GET /api/items/ (Domain\Avatar\AvatarSlots, kept equal by its tests).
 */
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
/** `food` = eaten with the `apply` skill ("Essen" instead of "Benutzen"), e.g. the cucumber and the tuna can lower appetite;
 *  `score` = sheet music (`ItemDef.score`, research/SCORES.md); `furniture` = stands in a home (`ItemDef.furniture`, homes.ts);
 *  `bag` = a carried container (a CD case, a DJ trolley, a record bag – `ItemDef.holds`, research/PACKING.md) */
export type ItemCategory = 'tool' | 'consumable' | 'wearable' | 'collectible' | 'media' | 'substance' | 'food' | 'score' | 'furniture' | 'bag';
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

/** "Nach Hause" from anywhere – the VR headset (research/CAVE.md D2): enters the reader's home without walking there */
export interface EnterHomeSkill {
	type: 'enter_home';
}

export type Skill = ScanQrSkill | ScanObjectSkill | MinigameSkill | PlayAudioSkill | ApplySkill | EnterHomeSkill;

/**
 * A shared base model built in code (owner 2026-10-16: "dass wir insgesamt auch ganz oft Shared 3D-Modelle anlegen
 * können"): the record and its sleeve are the same for every LP – an item gives only what differs, its pictures (CDN-root
 * WebP paths) and colour (ITEMS.md "3D item models"). `@scutoons/media/vinyl-model` builds it.
 */
export interface VinylLook {
	/** the base model: `lp12` = a 12" LP (Ø 302 mm) – a 7" single, a cassette … follow as further base models */
	format: 'lp12';
	/** the printed sleeve's front (~1024 px); none = a plain paper inner sleeve (a test press) */
	sleeveFront?: string;
	/** its back (~768 px) and spine (a narrow strip); none = plain card */
	sleeveBack?: string;
	sleeveSpine?: string;
	/** the labels, square pictures of the round label (~512 px); none = a blank white label */
	labelA?: string;
	labelB?: string;
	/** the vinyl's colour `#rrggbb`; none = black */
	color?: string;
}

/** what an item shows in 3D: a `.glb` (CDN-root path) or a shared base model's look */
export type ModelSource = string | VinylLook;

/** the formats of `VinylLook` (code – the base models the viewer can build) */
export const VINYL_FORMATS = ['lp12'] as const;

export interface ItemVariant {
	id: string;
	name: Localized;
	icon?: string | null;
	/** this variant's own 3D model (CDN-root `.glb`); none = the item's `model` */
	model?: string | null;
	/** this variant's tint while worn (owner 2026-10-08: sunglasses in six colours as variants); none = the item's `worn_tint` */
	worn_tint?: string | null;
	/** this variant's wardrobe overlays (`ItemDef.wear`'s keys); none = the item's */
	wear?: Record<string, string> | null;
}

export interface ItemDef extends AcceptedWarnings {
	id: string;
	toon: string | null;
	name: Localized;
	description: Localized;
	kind: ItemKind;
	category: ItemCategory;
	icon: string;
	/**
	 * The item's 3D model (FEATURES 10.102, owner 2026-10-16: "We will turn all items into 3d assets"): a CDN-root `.glb`
	 * (ITEMS.md "3D item models" – metres, +Y up, front +Z). The item detail shows it instead of the icon (WebGL, no
	 * reduced motion); a variant may have its own. The LP's is its sleeve with the record inside (nodes `sleeve`,
	 * `record`) – the record player takes the record out (record-stage.js). None = the icon only.
	 */
	model?: string | null;
	/** the record and sleeve as the shared base model (a look instead of a whole `.glb`); it wins over `model` */
	vinyl?: VinylLook | null;
	stackable: boolean;
	starter: boolean;
	grantable: GrantSource[];
	/** a story may take it from the reader (VN `takeItem` / a choice's `take`; owner 2026-10-07, BACKLOG 10.38) – never a
	 *  physical item; off by default */
	takeable?: boolean;
	slot: AvatarSlot | null;
	skills: Skill[];
	edition_id: string | null;
	product: string | null;
	variants: ItemVariant[] | null;
	/**
	 * Wardrobe overlays of a wearable (ITEMS.md "Wardrobe"): `<character>` or `<character>/<slot instance id>` → CDN
	 * path of a full-body overlay aligned to that character's A-pose (768 × 1152, transparent WebP, convention
	 * `characters/<character>/wear/<item>[-<slot instance>].webp`). Not drawn since 2026-10-12 (items sit in the avatar
	 * page's slot tiles, ITEMS.md "Wardrobe"); kept for the dress-up research, it only picks the basic dress.
	 */
	wear?: Record<string, string> | null;
	/**
	 * Part of the basic dress (ITEMS.md "Wardrobe": T-shirt, shorts, sandals): a starter wearable that is put on when it
	 * is handed out (signup, older accounts, guests). A character without an overlay for it wears another starter of the same
	 * slot that has one instead (the swim pants on yellowmouse / Top Toy) – `basicDress`.
	 */
	default_worn?: boolean;
	/**
	 * When a picked-up placement of this item is back on its slice (respawn.ts): a default interval and the range a
	 * placement may choose; none / null = it never reappears (a one-time find). Admin → Catalog can override it.
	 */
	respawn?: ItemRespawn | null;
	/**
	 * the sound when a picked-up placement lands in the bag (agents/FX.md) – none = "pling", `none` = silent; a
	 * placement can choose its own (`SliceItem.sound`)
	 */
	sound?: FxSoundChoice;
	/** a substance's stat changes (`catalog.stats` ids), ITEMS.md "Substances" */
	effects?: StatEffect[] | null;
	/** a substance's temporary reader effect */
	effect?: TimedEffect | null;
	/**
	 * sobers up (owner 2026-10-08: "eating candy reduces the effect to half the time … lots of cucumbers reduce the time
	 * it lasts"): each piece used takes this share (0 < x < 1) off the remaining time of every running reader effect –
	 * 0.5 halves it. An item with it is used like a substance (`apply` skill).
	 */
	sober?: number | null;
	/**
	 * a wearable that tints what the reader sees while it is worn (owner 2026-10-08: "putting on some cool sunglasses will
	 * make everything orange hued"): a colour `#rrggbb`, laid over everything (multiply)
	 */
	worn_tint?: string | null;
	/**
	 * Collection limit: at most this many per account (owner 2026-10-02); none = `MAX_PER_ACCOUNT` (99). Rarer items set
	 * less. The server refuses more (409 max_reached); a claimed physical product (QR code) is always added.
	 */
	max?: number;
	/**
	 * Item limits across the system (BACKLOG 10.39, ITEMS.md "Item limits"): how often it may be placed on slices, in
	 * every toon together (the creator: a Problems error, a paste refuses; the API: 422 item_limit on a save that adds
	 * one over it) – none = unlimited.
	 */
	placements?: number | null;
	/**
	 * How many accounts may own it at once (1 = it exists only once in the world); the server refuses anyone else
	 * (409 taken) while that many own it – none = unlimited.
	 */
	max_world?: number | null;
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
	/** A container (containers.ts, research/SCORES.md §2b): what it holds, how many, an overfill that breaks it */
	holds?: ItemHolds | null;
	/** It ages from its creation: tint, then spots (the DIY song book; containers.ts `ageLook`) */
	ages?: ItemAges | null;
	/** A score's files – MusicXML for the score view, the print PDF (one language per score) */
	score?: ScoreFiles | null;
	/** Furniture: its size and picture in a home, a workbench (homes.ts, research/CAVE.md D4) */
	furniture?: ItemFurniture | null;
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
	/** an input that is kept – a tool like the drill, not an ingredient (research/SCORES.md §2a) */
	keep?: boolean;
	/** the output only: its variant is taken from this input item's variant (the plastic's colour, an upgraded book's) */
	variant_from?: string;
}

export interface Recipe {
	id: string;
	inputs: RecipePart[];
	output: RecipePart;
	/** the recipe's own picture (CDN path, `<toon>/items/recipes/<id>.webp`; owner 2026-10-04) – none = the output item's */
	image?: string;
	/** where it can be made: `workshop` = only at a workbench in a home (the broken song book's repair, research/CAVE.md D4) */
	at?: 'workshop';
}

/** The item catalog. `accepted_warnings` (here and on items / albums) are the admin's accepted catalog warnings
 *  (catalogWarningPlace): stored with the entries – a list-wide one in the meta entry `items` – never served to readers. */
export interface Catalog extends AcceptedWarnings {
	version: number;
	slots: AvatarSlot[];
	items: ItemDef[];
	recipes: Recipe[];
	/** the reader stats substances change (`testo`, `appetite`): baseline + half-life, ITEMS.md "Substances" */
	stats?: Record<string, StatDef>;
	/** the music media items carry (albums.ts), language-neutral */
	albums?: Album[];
	/** what homes look like (homes.ts, research/CAVE.md): `cave`, `tatami_room` … */
	home_scenes?: HomeScene[];
	/** the creature airlift's carriers – one at random per flight (homes.ts, research/CAVE.md D8) */
	airlift_carriers?: AirliftCarrier[];
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
	/** the container entry it lies in (containers.ts; none = loose in the inventory) */
	in?: EntryId | null;
	/** a container: overfilled since (it breaks after `holds.overfill.days`) */
	overfilled_at?: string | null;
	/** an ageing item ages from here (none = `acquired_at`; an upgraded book keeps the old age) */
	aged_since?: string | null;
	/** furniture standing in a home (homes.ts; it stays there – research/CAVE.md D4) */
	home?: EntryHome | null;
}

/** Where an avatar-body unlock came from (server `member_characters.source`) */
export type UnlockSource = 'default' | 'code' | 'item' | 'story' | 'admin';

/** A character the member may wear as avatar body, with the unlocked looks (`/me/ avatar.unlocked`) */
export interface CharacterUnlock {
	character: string;
	/**
	 * unlocked looks with an A-pose as `<persona>/<look>` (`bodyLookKey`; the server lists the default body too – a
	 * character unlock always includes it)
	 */
	looks: string[];
	source: UnlockSource;
}

/** The chosen avatar body: a look (of a persona of a character) with an A-pose */
export interface AvatarBodyRef {
	character: string;
	persona: string;
	look: string;
}

/**
 * The default unlocks of every member and of guests (owner 2026-10-05: Cat Pope – its default body). The server's value
 * is the setting AVATAR_DEFAULT_UNLOCKS (members get theirs from `/me/`); guests use this one.
 */
export const DEFAULT_AVATAR_UNLOCKS: CharacterUnlock[] = [{ character: 'catpope', looks: [], source: 'default' }];

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
	/** "Show hints" (off by default; `PUT /me/ { show_hints }`): the reader outlines what can be tapped (BACKLOG M.1) */
	show_hints?: boolean;
	/** "Media quality" (`auto` by default; `PUT /me/ { media_quality }`) – media-quality.ts */
	media_quality?: 'auto' | 'low' | 'medium' | 'high';
	/** the reader's name (askText scope "reader", `{reader.name}`; `PUT /me/ { display_name }`, at most 32 characters) – null = none */
	display_name?: string | null;
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
