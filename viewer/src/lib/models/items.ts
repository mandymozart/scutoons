// Items, inventory, avatar — contract: agents/ITEMS.md (served by qr.scutoons.com /api)

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
export type ItemCategory = 'tool' | 'consumable' | 'wearable' | 'collectible' | 'media';
export type GrantSource = 'story' | 'scan';
export type EntrySource = 'qr' | 'story' | 'scan' | 'craft' | 'starter';

export interface ScanQrSkill {
	type: 'scan_qr';
}

export interface ScanObjectSkill {
	type: 'scan_object';
	labels: string[];
	grants: string;
	cooldown_s?: number;
	threshold?: number;
}

export interface MinigameSkill {
	type: 'minigame';
	id: string;
}

export interface PlayAudioSkill {
	type: 'play_audio';
	src: string;
}

export type Skill = ScanQrSkill | ScanObjectSkill | MinigameSkill | PlayAudioSkill;

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

export interface Avatar {
	slots: Partial<Record<AvatarSlot, EntryId | null>>;
}

export interface User {
	id: number | string;
	email: string;
	created_at: string;
}

export interface ServerMembership {
	code: string;
	type: string;
	expires_at: string | null;
}

export interface Me {
	user: User;
	memberships: ServerMembership[];
	inventory: Entry[];
	avatar: Avatar;
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
