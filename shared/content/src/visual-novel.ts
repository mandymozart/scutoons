import type { JumpToTarget, Onomatopoeia } from './content.ts';
import type { Trigger, TriggerMode } from './enums.ts';

/**
 * One mood of a character in its three views (framings). "View" is the term in the UI (never "variant", which would be
 * confused with looks); the keys and `VNCharacterRef.variant` keep their names for backwards compatibility.
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
	/** the persona's own color (else the character's) */
	color?: string;
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

export interface VNChoice {
	label: string;
	onSelect?: VNVariableSetter | VNVariableAdder | VNJumpTo;
}

export interface Message {
	character: SceneCharacter | null;
	text: string;
	wait: boolean;
}

/** A view (framing) of a character in a scene. */
export type CharacterView = 'avatar' | 'full' | 'portrait';

/**
 * A character reference in a VN event (joinScene / leaveScene), as authored: `{ character, persona, look?, mood?,
 * variant? }` (`variant` = the view). The old form `{ name, position: { variant } }` and old character ids (plf, cat,
 * mountain-tiger → catpope personas) still resolve (`normalizeSceneRef` in characters.ts). The same character may be in
 * a scene with several personas (Cat Pope meets Cat Pope · Biker).
 */
export interface VNCharacterRef {
	/** the character id */
	character?: string;
	/** a persona of the character (none = `regular`) */
	persona?: string;
	/** a look of the persona (e.g. `helmet`); none = its default look */
	look?: string;
	mood?: string;
	/** the view (avatar / portrait / full) */
	variant?: CharacterView;
	/** @deprecated old form: the character id */
	name?: string;
	/** @deprecated old form: the view */
	position?: {
		variant: CharacterView;
	};
}

/** A character on stage (the reader's runtime form of a ref): the scene key is `name` (character id) + persona. */
export interface SceneCharacter {
	/** the character id */
	name: string;
	persona?: string;
	look?: string;
	mood?: string;
	position: {
		variant: CharacterView;
	};
	/** set while a look changes (a transition image shown briefly) */
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
	/**
	 * Id of a text box (`Dialog.id`) – on the event's slice, or for a group script on one of the group's slices. That box
	 * stays hidden until the event fires, then appears with the event's effect / duration / delay / wait and stays; the box
	 * gives the look (variant, font, colour, outline, rotation, scale, per-language text), the event the timing. `text`,
	 * `position`, `size`, `align`, `rotation`, `scale` and `variant` of the event are then not used. Without it: the
	 * event's own onomatopoeia, as before.
	 */
	dialog?: string;
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
