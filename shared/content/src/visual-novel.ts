import type { JumpToTarget } from './content.ts';
import type { TextBoxEffect, Trigger, TriggerMode } from './enums.ts';
import type { Repeat } from './recurrence.ts';

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
	description?: string;
	collection?: string;
	/** mood → view → path: the regular persona's default look (what readers without persona support show) */
	images?: Record<string, CharacterImage>;
	/** every persona (registry characters); absent in the old static files */
	personas?: Record<string, ResolvedPersona>;
}

export interface VNChoice {
	label: string;
	onSelect?: VNAction | VNAction[];
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

/** continue the group's script at a named point (`label` event) – the story's own branch, no page change */
export interface VNDivert {
	divert: {
		to: string;
	};
}

/** give the reader a catalog item (BACKLOG 10.38: an item is never a story variable) – the inventory's rules apply:
 *  the item must be grantable from the story, its max count, once per place (a replay grants nothing) */
export interface VNGiveItem {
	give: {
		/** a catalog item id (picked from the catalog – a script can't make items) */
		item: string;
		/** how many, 1 … 10 (default 1) */
		qty?: number;
	};
}

/**
 * take a catalog item from the reader (owner 2026-10-07, BACKLOG 10.38): only an item the catalog marks `takeable`
 * (never a physical one); all or nothing – with too few nothing is taken (and in a choice's list the actions after it
 * don't run: a trade never gives for free); the story goes on. `repeat`: how often it takes again (the trigger
 * intervals; none = every time).
 */
export interface VNTakeItem {
	take: {
		item: string;
		/** how many, 1 … 10 (default 1) */
		qty?: number;
		repeat?: Repeat;
	};
}

/** what a choice does when picked – one action or several in order (e.g. add 15 minutes, then jump) */
export type VNAction = VNVariableSetter | VNVariableAdder | VNGiveItem | VNTakeItem | VNJumpTo | VNDivert;

/**
 * "Only if" on any event (owner 2026-10-03): the event runs only when the reader's variable passes the test, else it is
 * skipped. Numbers compare as numbers; `set` / `unset` ask whether the variable has a value at all.
 */
export type VNConditionOp = '==' | '!=' | '<' | '<=' | '>' | '>=' | 'set' | 'unset';
export const VN_CONDITION_OPS: VNConditionOp[] = ['==', '!=', '<', '<=', '>', '>=', 'set', 'unset'];
export interface VNCondition {
	variable: string;
	op: VNConditionOp;
	value?: string | number;
}

/**
 * "Only if" as an expression (agents/EXPRESSIONS.md §3, the advanced form): ink's logic on the expression engine and the
 * reader's resolver – `count("s1c2-e4k2j9x1") < 3 and not has("cucumber")`. A syntax error can't be saved.
 */
export interface VNExprCondition {
	expr: string;
}

/** an event's `if`: the simple test (the creator's default) or an expression */
export type VNEventCondition = VNCondition | VNExprCondition;

/** a named point in a group's script – a divert (event, choice action, slice trigger) continues here */
export interface VNLabelEvent {
	type: 'label';
	params: { name: string };
}

/** continue at a named point of the same script */
export interface VNDivertEvent {
	type: 'divert';
	params: { to: string };
}

/** set a variable, or add to it (numbers) */
export interface VNSetVariableEvent {
	type: 'setVariable';
	params: { variable: string; value: string | number; op?: 'set' | 'add' };
}

/** give the reader a catalog item (VNGiveItem's rules) – the script's way; a choice has the action `give` */
export interface VNGiveItemEvent {
	type: 'giveItem';
	params: VNGiveItem['give'];
}

/** take a catalog item from the reader (VNTakeItem's rules) – the script's way; a choice has the action `take` */
export interface VNTakeItemEvent {
	type: 'takeItem';
	params: VNTakeItem['take'];
}

export interface VNShowMessageEvent {
	type: 'showMessage';
	params: {
		text: string;
		/** the speaking character's id; none = narration (no speaker) */
		character?: string;
		/** the speaker's persona (none = regular) */
		persona?: string;
		/** the speaker's mood while saying it (the creator writes it; none = the mood they are in) */
		mood?: string;
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
		onSelect?: VNAction | VNAction[];
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

/**
 * A text box revealed by this event (owner 2026-10-03, option B; any text box, e.g. an onomatopoeia): `dialog` is the id
 * of a text box (`Dialog.id`) on the event's slice, or for a group script on one of the group's slices. The box stays hidden until the
 * event fires, then appears with `effect` / `delay` / `duration` and stays; the box gives the look (variant, font,
 * colour, outline, rotation, scale, text per language), the event the timing.
 */
export interface ShowTextBoxParams {
	dialog: string;
	/** default 'none' */
	effect?: TextBoxEffect;
	/** ms the effect plays, default 2000 (max 5000) */
	duration?: number;
	/** ms before the box appears, default 0 (max 5000) */
	delay?: number;
	/** `mono` = one at a time (queued), `poly` = together; default poly */
	mode?: TriggerMode;
	/** the script waits for a tap before it continues */
	wait?: boolean;
	/** after its effect the box stays (default true) or disappears again (`false`, after `delay` + `duration`) */
	stay?: boolean;
}

export interface VNShowTextBoxEvent {
	type: 'showTextBox';
	params: ShowTextBoxParams;
}

/**
 * A question with one input line in the message box (agents/EXPRESSIONS.md §6): the reader types a short plain text
 * (trimmed, never markup), the page doesn't scroll until it is answered (Enter / ✓, or the answer button: `answer`
 * is its text, `default` what it stores – owner 2026-10-06). `scope`:
 * `toon` (default) = a story variable (`variable` – since 2026-10-06 the same in every toon: one save game, so the name
 * `toon` only means "the story's" – kept, no migration); `reader` = the reader's name (`reader.name`: the account's
 * `display_name` for members, the device for guests; changeable on the account page – not part of the save game).
 */
export interface VNAskTextEvent {
	type: 'askText';
	params: {
		/** the question */
		text: string;
		/** who asks (none = narration) */
		character?: string;
		persona?: string;
		/** `toon` scope: the story variable the answer goes to */
		variable?: string;
		/** a hint in the empty input line */
		placeholder?: string;
		/** at most this many characters (default 24, at most 200) */
		maxLength?: number;
		/** a ready answer as a button under the input line: its text (none = no button) */
		answer?: string;
		/** what the answer button (or an empty answer) stores; none = nothing is stored – the variable stays unset, so a
		 *  later condition can tell it apart */
		default?: string;
		/** default `toon` */
		scope?: AskTextScope;
	};
}

export const ASK_TEXT_SCOPES = ['toon', 'reader'] as const;
export type AskTextScope = (typeof ASK_TEXT_SCOPES)[number];
/** an answer's default / largest length */
export const ASK_TEXT_MAX_LENGTH = 24;
export const ASK_TEXT_LIMIT = 200;
/** the most pieces one `giveItem` / `give` hands out (the inventory grants one piece per call – BACKLOG 10.38) */
export const GIVE_ITEM_MAX = 10;

export interface VNJumpToEvent {
	type: 'jumpTo';
	params: {
		target: JumpToTarget;
		wait?: boolean;
	};
}

export type VNEvent = (
	| VNShowMessageEvent
	| VNShowChoicesEvent
	| VNAddChoiceEvent
	| VNJoinSceneEvent
	| VNLeaveSceneEvent
	| VNHideMessageEvent
	| VNShowTextBoxEvent
	| VNJumpToEvent
	| VNLabelEvent
	| VNDivertEvent
	| VNSetVariableEvent
	| VNGiveItemEvent
	| VNTakeItemEvent
	| VNAskTextEvent
) & {
	/**
	 * hidden stable id (the creator assigns it, unique in the chapter and its scenes – like a text box id): the reader's
	 * save game remembers the event by it (`event:<id>`: repeat, `resolved(id)` / `count(id)`)
	 */
	id?: string;
	/** "Only if": the event runs only when this holds (skipped otherwise) */
	if?: VNEventCondition;
};
	
// enhanced types for slice triggered
export interface VNEventTrigger {
	type: Trigger;
	/**
	 * how often the trigger runs its events (recurrence.ts): none = every time; once per page view, once for good (the
	 * save game), once per n minutes / hours / days. Events with the same trigger run as one.
	 */
	repeat?: Repeat;
	rect?: {
		x: number;
		y: number;
		width: number;
		height: number;
	};
}

export type VNSliceEvent = VNEvent & {
	trigger: VNEventTrigger;
	/**
	 * a click / hover area's place in the slice's ONE layer stack (layers.ts, BACKLOG 10.54): higher = in front (drawn on
	 * top, gets the tap / hover first); the events sharing the trigger carry the same number. None = the old order.
	 */
	layer?: number;
}

export interface VNEventsConfig {
	events?: VNSliceEvent[];
	onNavigate?: (url: string) => void;
}
