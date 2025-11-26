import type { Onomatopoeia } from "./content.ts";
import type { OnomatopoeiaEffect, OnomatopoeiaVariant, TriggerMode } from "./enums.ts";

export interface CharacterImage {
	avatar?: string;
	portrait?: string;
	full?: string;
}

export interface Character {
	name: string;
	label: string;
	color: string;
	image: string; // depreacted for images slots
	description?: string;
	collection?: string;
	images?: Record<string, CharacterImage>;
}

export interface CollectableRef {
	id: string;
	count: number;
	expiration_date?: string; // does not expire if null
}

export interface Collectable {
	id: string;
	description: string;
	stock?: number; // unlimited if null
	life_expectency?: number; // unlimited if null
}

export interface VNChoice {
	label: string;
	onSelect?: VNVariableSetter | VNVariableAdder;
}

export interface Message {
	character: VNCharacterRef | null;
	text: string;
	wait: boolean;
}

export interface VNCharacterRef {
	name: string;
	position?: {
		variant: 'avatar' | 'full' | 'portrait';
	};
	mood?: string;
}

export interface VNVariableSetter {
	set: {
		variable: string;
		value: string;
	};
}

export interface VNVariableAdder {
	add: {
		variable: string;
		value: number;
	};
}

export interface VNShowMessageEvent {
	type: 'showMessage';
	params: {
		text: string;
		character: string;
		wait?: boolean;
	};
}

export interface VNShowChoicesEvent {
	type: 'showChoices';
	params?: {};
}

export interface VNAddChoiceEvent {
	type: 'addChoice';
	params: {
		label: string;
		onSelect?: VNVariableSetter | VNVariableAdder;
	};
}

export interface VNJoinSceneEvent {
	type: 'joinScene';
	params: {
		characters: VNCharacterRef[];
	};
}

export interface VNLeaveSceneEvent {
	type: 'leaveScene';
	params: {
		characters: VNCharacterRef[];
	};
}

export interface VNHideMessageEvent {
	type: 'hideMessage';
	params?: {};
}

type OnomatopoeiaParams = Onomatopoeia & {
	wait?: boolean;
	mode?: TriggerMode;
};

export interface VNShowOnomatopeiaEvent {
	type: 'showOnomatopeia';
	params: OnomatopoeiaParams; // shares config with webtoon layout for convenience
}

export interface VNEndEvent {
	type: 'end';
	params: {
		jumpTo: string;
	};
}

export type VNEvent =
	| VNShowMessageEvent
	| VNShowChoicesEvent
	| VNAddChoiceEvent
	| VNJoinSceneEvent
	| VNLeaveSceneEvent
	| VNHideMessageEvent
	| VNShowOnomatopeiaEvent
	| VNEndEvent;
