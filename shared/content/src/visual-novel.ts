import type { JumpToTarget, Onomatopoeia } from './content.ts';
import type { Trigger, TriggerMode } from './enums.ts';

/**
 * One mood of a character in its three views (framings). "View" is the term in the UI (never "variant", which would be
 * confused with looks); the keys and `VNCharacterRef.position.variant` keep their names for backwards compatibility.
 */
export interface CharacterImage {
	avatar?: string;
	portrait?: string;
	full?: string;
}

/** A persona of a resolved character (registry `personas`, see characters.ts `resolveCast`). */
export interface ResolvedPersona {
	/** the shown name, e.g. "Cat Pope · Biker" */
	label: string;
	defaultLook: string;
	/** look → mood → view → path */
	looks: Record<string, Record<string, CharacterImage>>;
	/** images shown while the look changes */
	transitions?: { from: string; to: string; image: string }[];
}

/**
 * A character as the VN renderer reads it (`<toon>/<locale>/characters.json` shape; from the registry via
 * `resolveCast`). Images: `legacyCharacterImage` (characters.ts) applies the fallback rule.
 */
export interface Character {
	/** the character id */
	name: string;
	label: string;
	color: string;
	/** the legacy single image (last fallback) */
	image: string;
	description?: string;
	collection?: string;
	/** mood → view → path: the regular persona's default look (what readers without persona support show) */
	images?: Record<string, CharacterImage>;
	/** every persona (registry characters); absent in the old static files */
	personas?: Record<string, ResolvedPersona>;
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

/**
 * A character in a scene: `name` = the character id (legacy refs with only a name = the regular persona), optional
 * `persona`, `look` and `mood`; `position.variant` = the view (avatar / portrait / full). The same character may be in
 * a scene with several personas (Cat Pope meets Cat Pope · Biker): the scene key is name + persona.
 */
export interface VNCharacterRef {
	name: string;
	/** a persona of the character (none = `regular`) */
	persona?: string;
	/** `variant` = the view (avatar / portrait / full) */
	position?: {
		variant: 'avatar' | 'full' | 'portrait';
	};
	mood?: string;
	/** a look of the character (e.g. `helmet`); none = its default look */
	look?: string;
	/** set by the reader while a look changes (a transition image shown briefly) – never authored */
	transition?: string;
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
		/** the speaking character's id */
		character: string;
		/** the speaker's persona (none = regular) */
		persona?: string;
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
