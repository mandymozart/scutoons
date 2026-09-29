import type { JumpToTarget, Onomatopoeia } from './content.ts';
import type { Trigger, TriggerMode } from './enums.ts';

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
	/** mood → variant → path (the default look) */
	images?: Record<string, CharacterImage>;
	/** look → mood → variant → path, for characters with several looks (registry `looks`) */
	looks?: Record<string, Record<string, CharacterImage>>;
	/** the look used when a ref names none */
	defaultLook?: string;
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
	onSelect?: VNVariableSetter | VNVariableAdder | VNJumpTo;
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
	/** a look of the character (e.g. `helmet`); none = its default look */
	look?: string;
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

export interface VNJumpTo {
	jumpTo: {
		target: JumpToTarget;
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
	mode?: TriggerMode;
	wait?: boolean;
};

export interface VNShowOnomatopeiaEvent {
	type: 'showOnomatopeia';
	params: OnomatopoeiaParams; // shares config with webtoon layout for convenience
}

export interface VNJumpToEvent {
	type: 'jumpTo';
	params: {
		target: JumpToTarget;
		wait?: boolean;
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
	| VNJumpToEvent;
	
// enhanced types for slice triggered
export interface VNEventTrigger {
	type: Trigger;
	once?: boolean;
	rect?: {
		x: number;
		y: number;
		width: number;
		height: number;
	};
}

export type VNSliceEvent = VNEvent & {
	trigger: VNEventTrigger
}

export interface VNEventsConfig {
	events?: VNSliceEvent[];
	onNavigate?: (url: string) => void;
}
