import { setContext, getContext } from 'svelte';
import type { Character, VNChoice, VNEvent, Message, VNCharacterRef, SceneCharacter } from '@scutoons/content';
import { LEGACY_CHARACTER_IDS, REGULAR_PERSONA, legacyTransition, normalizeSceneRef } from '@scutoons/content';
import type { OnomatopoeiaParams } from '@scutoons/content';
import type { TriggerMode } from '@scutoons/content';
import type { ActiveOnomatopoeia } from '@scutoons/reader';

const VN_KEY = Symbol('visual-novel');

/** How long a look transition image is shown before the new look (ms). */
const TRANSITION_MS = 800;

/** A character in a scene is identified by id + persona (Cat Pope may meet Cat Pope · Biker). */
export const sceneKey = (ref: Pick<SceneCharacter, 'name' | 'persona'>) => `${ref.name}:${ref.persona || REGULAR_PERSONA}`;

/** An onomatopoeia event playing: its text box is drawn by its slice (`dialogPlaying`). */
export type VNActiveOnomatopoeia = ActiveOnomatopoeia;

export type VisualNovelContext = ReturnType<typeof createVisualNovelContext>;

let _currentContext: VisualNovelContext | undefined;

export function createVisualNovelContext() {
	const characters = $state(new Map<string, Character>());
	const sceneCharacters = $state<SceneCharacter[]>([]);
	const pendingChoices = $state<VNChoice[]>([]);
	const isProcessingEvents = $state(false);
	const activeOnomatopoeia = $state<VNActiveOnomatopoeia[]>([]);
	const onomatopoeiaQueue = $state<VNActiveOnomatopoeia[]>([]);
	/**
	 * Text boxes that onomatopoeia events reveal (`params.dialog`, agents/DESIGN.md "Onomatopoeia → text box"): the ids a
	 * group script refers to (hidden until revealed; slice events are known by their slice) and the ids already revealed
	 * on this page view (they stay).
	 */
	const dialogRefs = $state<Record<string, true>>({});
	const revealedDialogs = $state<Record<string, true>>({});

	let messageVisible = $state(false);
	const message = $state<Message>({
		character: null,
		text: '',
		wait: false
	});
	let choicesVisible = $state(false);
	/** the dialogue box's height (px): the characters stand on its top edge */
	let boxHeight = $state(0);
	/** scene key → when it last spoke (a counter), for the stage order: the speaker, then by recency (vn/characters) */
	const lastSpoke = $state<Record<string, number>>({});
	let spokeCounter = 0;
	let waitResolver: (() => void) | null = null;
	/** the reader has clicked the visible message away (its wait is answered) – a following `jumpTo` (wait) jumps at once */
	let acknowledged = false;
	/** the group's waiting message while a slice trigger (e.g. a cucumber offer) uses the box, see `interrupt` */
	let suspended: { character: SceneCharacter | null; text: string; wait: boolean; resolver: () => void; acknowledged: boolean } | null = null;
	let onomatopoeiaResolver: (() => void) | null = null;
	let navigationTriggered = $state(false);
	/** the group's script ran to its end without navigating (the follow-up bar waits for it, agents/DESIGN.md) */
	let eventsDone = $state(false);
	/**
	 * The page's group script and its label (CUTOVER 10.27): a slice event of type divert runs the script from a named
	 * point; a random jump never picks the group the reader is on. A choice's divert waits here for the script.
	 */
	let script: VNEvent[] = [];
	let scriptLabel: string | null = null;
	let pendingDivert: string | null = null;

	function setCharacters(data: Character[]) {
		characters.clear();
		data.forEach((char) => {
			characters.set(char.name, char);
		});
		console.log(`Loaded ${characters.size} characters`);
	}

	function getCharacter(name: string): Character | undefined {
		return characters.get(name);
	}

	/**
	 * An authored ref (`{ character, persona, look?, mood?, variant? }` or the old `{ name, position }`) as a character on
	 * stage. Old ids (plf …) become their persona (catpope · human); when the loaded characters are the old static
	 * characters.json (no personas), a persona ref goes the other way (catpope · human → plf).
	 */
	function toScene(ref: VNCharacterRef): SceneCharacter | null {
		const n = normalizeSceneRef(ref);
		if (!n) return null;
		let name = n.character;
		let persona = n.persona;
		if (!characters.get(name)?.personas) {
			const legacy = Object.entries(LEGACY_CHARACTER_IDS).find(([id, t]) => t.character === name && t.persona === persona && characters.has(id));
			if (legacy) {
				name = legacy[0];
				persona = REGULAR_PERSONA;
			} else if (!characters.has(name) && characters.has(ref.character ?? ref.name ?? '')) {
				name = (ref.character ?? ref.name) as string;
				persona = REGULAR_PERSONA;
			}
		}
		return {
			name,
			...(persona !== REGULAR_PERSONA ? { persona } : {}),
			...(n.look ? { look: n.look } : {}),
			...(n.mood ? { mood: n.mood } : {}),
			position: { variant: (n.variant as SceneCharacter['position']['variant']) ?? 'avatar' }
		};
	}

	/** The character on stage for a speaker (+ persona; without a persona: the regular one, else any persona of it). */
	function findInScene(id: string, persona?: string): SceneCharacter | undefined {
		const target = toScene({ character: id, persona });
		if (!target) return undefined;
		if (persona) return sceneCharacters.find((c) => sceneKey(c) === sceneKey(target));
		return sceneCharacters.find((c) => sceneKey(c) === sceneKey(target)) ?? sceneCharacters.find((c) => c.name === target.name);
	}


	function joinScene(refs: VNCharacterRef[]): void {
		const charsToAdd: SceneCharacter[] = [];
		const chars = refs.map(toScene).filter((c): c is SceneCharacter => !!c);
		
		for (let newChar of chars) {
			const existingIndex = sceneCharacters.findIndex((c) => sceneKey(c) === sceneKey(newChar));

			// a newcomer hasn't spoken yet (counts as the longest silent)
			if (existingIndex === -1) delete lastSpoke[sceneKey(newChar)];
			if (existingIndex !== -1) {
				const existing = sceneCharacters[existingIndex];
				const existingVariant = existing.position?.variant;
				const newVariant = newChar.position?.variant;
				const existingMood = existing.mood;
				const newMood = newChar.mood;

				if (existingVariant !== newVariant || existingMood !== newMood || existing.look !== newChar.look) {
					// a look change with a transition image (helmet → default): show it briefly first
					const transition = existing.look !== newChar.look ? legacyTransition(characters.get(newChar.name), newChar.persona, existing.look, newChar.look) : '';
					if (transition) {
						newChar = { ...newChar, transition };
						const key = sceneKey(newChar);
						setTimeout(() => {
							const ref = sceneCharacters.find((c) => sceneKey(c) === key);
							if (ref?.transition === transition) ref.transition = undefined;
						}, TRANSITION_MS);
					}
					console.log(
						`Character "${newChar.name}" already in scene (variant: "${existingVariant}", mood: "${existingMood}"), replacing with (variant: "${newVariant}", mood: "${newMood}")`
					);
					// Remove from current position, will be added in new order
					sceneCharacters.splice(existingIndex, 1);
					charsToAdd.push(newChar);
				} else {
					console.log(
						`Character "${newChar.name}" already in scene with same variant "${existingVariant}" and mood "${existingMood}", reordering`
					);
					// Remove from current position to add in new order
					sceneCharacters.splice(existingIndex, 1);
					charsToAdd.push(existing);
				}
			} else {
				charsToAdd.push(newChar);
			}
		}
		
		// Add all characters in the order specified
		sceneCharacters.push(...charsToAdd);
	}

	function leaveScene(refs: VNCharacterRef[]): void {
		// with a persona: that persona; without: every persona of the character
		const chars = refs.map((r) => ({ ref: r, scene: toScene(r) })).filter((x) => !!x.scene);
		const leaving = (c: SceneCharacter) => chars.some(({ ref, scene }) => (ref.persona || scene!.persona ? sceneKey(scene!) === sceneKey(c) : scene!.name === c.name));
		const filtered = sceneCharacters.filter((c) => !leaving(c));
		sceneCharacters.length = 0;
		sceneCharacters.push(...filtered);
	}

	function showMessage(charRef: SceneCharacter | null, text: string, wait: boolean): void {
		message.character = charRef;
		if (charRef) lastSpoke[sceneKey(charRef)] = ++spokeCounter;
		message.text = text;
		message.wait = wait;
		messageVisible = true;
		acknowledged = false;
	}

	function hideMessage(): void {
		messageVisible = false;
	}

	function setWaitResolver(resolver: () => void): void {
		waitResolver = resolver;
	}

	function continueFromWait(): void {
		if (waitResolver) {
			// the click answers this message: no "…" any more, and nothing else waits for the same click
			message.wait = false;
			acknowledged = true;
			const resolve = waitResolver;
			waitResolver = null;
			resolve();
		}
	}

	/**
	 * `jumpTo` with `wait`: jump once the reader is done with the dialogue box. One click per message – when the reader
	 * already clicked the last message away (or no message is shown), it resolves at once; a message shown without `wait`
	 * gets the "…" and one click.
	 */
	function waitForContinue(): Promise<void> {
		if (acknowledged || !messageVisible || choicesVisible) return Promise.resolve();
		message.wait = true;
		return new Promise<void>((resolve) => {
			waitResolver = resolve;
		});
	}

	/** A group's script starts (again): not done. */
	function beginEvents(): void {
		eventsDone = false;
	}

	/** The group's events are done: a message the reader already clicked away closes (the last click advances too). */
	function endOfEvents(): void {
		if (acknowledged && !choicesVisible) messageVisible = false;
		eventsDone = true;
	}

	function addChoice(choice: VNChoice): void {
		pendingChoices.push(choice);
	}

	/** choices a script added but never showed (its run ended): gone, so they can't join a later question (CUTOVER 10.27) */
	function dropUnshownChoices(count: number): void {
		if (choicesVisible || count <= 0) return;
		pendingChoices.splice(Math.max(0, pendingChoices.length - count), count);
	}

	function showChoices(): void {
		choicesVisible = true;
	}

	function hideChoices(): void {
		choicesVisible = false;
	}

	function clearChoices(): void {
		pendingChoices.length = 0;
		hideChoices();
		hideMessage();
		resumeIfIdle();
	}

	/**
	 * A slice trigger fires while the group's script waits for a click on its message: park that message (text + its
	 * wait) so the trigger's message / choices can use the box, `resumeIfIdle` brings it back afterwards.
	 */
	function interrupt(): void {
		if (suspended || !waitResolver || !messageVisible) return;
		suspended = { character: message.character, text: message.text, wait: message.wait, resolver: waitResolver, acknowledged };
		waitResolver = null;
	}

	/** Brings the parked message back once the trigger is done (no choices open, nothing waiting). */
	function resumeIfIdle(): void {
		if (!suspended || choicesVisible || pendingChoices.length || waitResolver) return;
		message.character = suspended.character;
		message.text = suspended.text;
		message.wait = suspended.wait;
		messageVisible = true;
		waitResolver = suspended.resolver;
		acknowledged = suspended.acknowledged;
		suspended = null;
	}

	function setScript(events: VNEvent[] | undefined, label: string | null): void {
		script = events ?? [];
		scriptLabel = label;
	}
	function setPendingDivert(to: string | null): void {
		pendingDivert = to;
	}
	/** the divert a picked choice asked for (once) */
	function takePendingDivert(): string | null {
		const to = pendingDivert;
		pendingDivert = null;
		return to;
	}

	function setNavigationTriggered(): void {
		navigationTriggered = true;
	}

	/** Choices navigate themselves (vn/choices.svelte → executeVariableAction); kept for the event processor. */
	function getPendingJumpTo(): string | null {
		return null;
	}

	function isNavigationTriggered(): boolean {
		return navigationTriggered;
	}


	/** Reveals the event's text box (`params.dialog`); an event without one shows nothing (the validation flags it). */
	function showOnomatopeia(params: OnomatopoeiaParams): boolean {
		if (typeof params?.dialog !== 'string' || !params.dialog) {
			console.warn('showOnomatopeia without params.dialog – nothing to reveal');
			return false;
		}
		const id = `ono-${Date.now()}-${Math.random()}`;
		const { wait = false, mode = 'poly' as TriggerMode, ...data } = params;

		const item: VNActiveOnomatopoeia = {
			...data,
			id,
			wait,
			mode,
			onComplete: () => handleOnomatopoeiaComplete(id)
		};

		const hasActiveMono = activeOnomatopoeia.some((o) => o.mode === 'mono');
		if (mode === 'mono' && hasActiveMono) {
			onomatopoeiaQueue.push(item);
		} else {
			activate(item);
		}
		return true;
	}

	/**
	 * Plays: the event's text box on its slice is revealed for good. The slice plays the box's effect and calls
	 * `onComplete`; a box that isn't on the page (another group, a typo) still completes after its time, so a `wait` /
	 * mono queue never hangs.
	 */
	function activate(item: VNActiveOnomatopoeia): void {
		activeOnomatopoeia.push(item);
		revealedDialogs[item.dialog] = true;
		const time = (item.delay ?? 0) + (item.duration ?? 2000);
		setTimeout(() => handleOnomatopoeiaComplete(item.id), Math.min(Math.max(time, 0), 10_000) + 1500);
	}

	/** a group script's text box refs (hidden until an event reveals them) */
	function registerDialogRefs(ids: string[]): void {
		for (const id of ids) dialogRefs[id] = true;
	}

	/** a text box waits for its onomatopoeia event (`sliceRefs` = the refs of its own slice's events) */
	function dialogHidden(id: string | undefined, sliceRefs?: ReadonlySet<string>): boolean {
		if (!id || revealedDialogs[id]) return false;
		return !!dialogRefs[id] || !!sliceRefs?.has(id);
	}

	/** the onomatopoeia event revealing this text box right now */
	function dialogPlaying(id: string | undefined): VNActiveOnomatopoeia | undefined {
		return id ? activeOnomatopoeia.find((o) => o.dialog === id) : undefined;
	}

	function handleOnomatopoeiaComplete(id: string): void {
		const index = activeOnomatopoeia.findIndex((o) => o.id === id);
		if (index !== -1) {
			const item = activeOnomatopoeia[index];
			activeOnomatopoeia.splice(index, 1);
			// `stay: false`: the box disappears again after its effect (hidden like before the event)
			if (item.stay === false) delete revealedDialogs[item.dialog];

			if (item.mode === 'mono' && onomatopoeiaQueue.length > 0) {
				const next = onomatopoeiaQueue.shift();
				if (next) {
					activate(next);
				}
			}

			const hasWaitingItems = activeOnomatopoeia.some((o) => o.wait) || 
				onomatopoeiaQueue.some((o) => o.wait);
			
			if (!hasWaitingItems && onomatopoeiaResolver) {
				onomatopoeiaResolver();
				onomatopoeiaResolver = null;
			}
		}
	}

	function setOnomatopoeiaResolver(resolver: () => void): void {
		onomatopoeiaResolver = resolver;
	}

	function continueFromOnomatopoeia(): void {
		activeOnomatopoeia.forEach((item) => item.onComplete());
		onomatopoeiaQueue.length = 0;
	}

	/**
	 * Resets the scene when a VN group ends. The characters stay: the toon layout loads them once (setCharacters), and the
	 * next VN group (another label or chapter of the same toon) needs them again.
	 */
	function clear(): void {
		sceneCharacters.length = 0;
		for (const key of Object.keys(lastSpoke)) delete lastSpoke[key];
		acknowledged = false;
		waitResolver = null;
		suspended = null;
		pendingChoices.length = 0;
		messageVisible = false;
		message.character = null;
		message.text = '';
		message.wait = false;
		choicesVisible = false;
		activeOnomatopoeia.length = 0;
		onomatopoeiaQueue.length = 0;
		for (const key of Object.keys(dialogRefs)) delete dialogRefs[key];
		for (const key of Object.keys(revealedDialogs)) delete revealedDialogs[key];
		navigationTriggered = false;
		eventsDone = false;
		pendingDivert = null;
	}

	return {
		get state() {
			return {
				characters,
				sceneCharacters,
				pendingChoices,
				isProcessingEvents,
				messageVisible,
				message,
				choicesVisible,
				activeOnomatopoeia,
				boxHeight,
				lastSpoke,
				eventsDone
			};
		},
		setCharacters,
		getCharacter,
		setBoxHeight: (h: number) => {
			boxHeight = h;
		},
		findInScene,
		joinScene,
		leaveScene,
		showMessage,
		hideMessage,
		setWaitResolver,
		continueFromWait,
		waitForContinue,
		beginEvents,
		endOfEvents,
		addChoice,
		dropUnshownChoices,
		showChoices,
		hideChoices,
		clearChoices,
		interrupt,
		resumeIfIdle,
		setNavigationTriggered,
		getPendingJumpTo,
		setScript,
		get script() {
			return script;
		},
		get scriptLabel() {
			return scriptLabel;
		},
		setPendingDivert,
		takePendingDivert,
		isNavigationTriggered,
		showOnomatopeia,
		registerDialogRefs,
		dialogHidden,
		dialogPlaying,
		setOnomatopoeiaResolver,
		continueFromOnomatopoeia,
		clear
	};
}

export function setVisualNovelContext() {
	const context = createVisualNovelContext();
	setContext(VN_KEY, context);
	_currentContext = context;
	return context;
}

export function getVisualNovelContext() {
	return getContext<VisualNovelContext>(VN_KEY);
}

export function getCurrentVisualNovelContext() {
	return _currentContext;
}
