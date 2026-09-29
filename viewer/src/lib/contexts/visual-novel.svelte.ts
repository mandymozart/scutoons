import { setContext, getContext } from 'svelte';
import type { Character, VNChoice, Message, VNCharacterRef } from '@scutoons/content';
import { REGULAR_PERSONA, legacyTransition } from '@scutoons/content';
import type { Onomatopoeia } from '@scutoons/content';
import type { TriggerMode } from '@scutoons/content';
import type { ActiveOnomatopoeia } from '@scutoons/reader';

const VN_KEY = Symbol('visual-novel');

/** How long a look transition image is shown before the new look (ms). */
const TRANSITION_MS = 800;

/** A character in a scene is identified by id + persona (Cat Pope may meet Cat Pope · Biker). */
export const sceneKey = (ref: Pick<VNCharacterRef, 'name' | 'persona'>) => `${ref.name}:${ref.persona || REGULAR_PERSONA}`;

/** An onomatopoeia on screen (rendered by `@scutoons/reader`'s `Onomatopoeia`). */
export type VNActiveOnomatopoeia = ActiveOnomatopoeia;

export type VisualNovelContext = ReturnType<typeof createVisualNovelContext>;

let _currentContext: VisualNovelContext | undefined;

export function createVisualNovelContext() {
	const characters = $state(new Map<string, Character>());
	const sceneCharacters = $state<VNCharacterRef[]>([]);
	const pendingChoices = $state<VNChoice[]>([]);
	const isProcessingEvents = $state(false);
	const activeOnomatopoeia = $state<VNActiveOnomatopoeia[]>([]);
	const onomatopoeiaQueue = $state<VNActiveOnomatopoeia[]>([]);

	let messageVisible = $state(false);
	const message = $state<Message>({
		character: null,
		text: '',
		wait: false
	});
	let choicesVisible = $state(false);
	let waitResolver: (() => void) | null = null;
	let onomatopoeiaResolver: (() => void) | null = null;
	let navigationTriggered = $state(false);

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

	/** The scene ref of a character (+ persona; without a persona: the regular one, else any persona of it). */
	function findInScene(name: string, persona?: string): VNCharacterRef | undefined {
		if (persona) return sceneCharacters.find((c) => sceneKey(c) === sceneKey({ name, persona }));
		return sceneCharacters.find((c) => sceneKey(c) === sceneKey({ name })) ?? sceneCharacters.find((c) => c.name === name);
	}

	function joinScene(chars: VNCharacterRef[]): void {
		const charsToAdd: VNCharacterRef[] = [];
		
		for (let newChar of chars) {
			const existingIndex = sceneCharacters.findIndex((c) => sceneKey(c) === sceneKey(newChar));

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

	function leaveScene(chars: VNCharacterRef[]): void {
		// with a persona: that persona; without: every persona of the character
		const leaving = (c: VNCharacterRef) => chars.some((l) => (l.persona ? sceneKey(l) === sceneKey(c) : l.name === c.name));
		const filtered = sceneCharacters.filter((c) => !leaving(c));
		sceneCharacters.length = 0;
		sceneCharacters.push(...filtered);
	}

	function showMessage(charRef: VNCharacterRef | null, text: string, wait: boolean): void {
		message.character = charRef;
		message.text = text;
		message.wait = wait;
		messageVisible = true;
	}

	function hideMessage(): void {
		messageVisible = false;
	}

	function setWaitResolver(resolver: () => void): void {
		waitResolver = resolver;
	}

	function continueFromWait(): void {
		if (waitResolver) {
			waitResolver();
			waitResolver = null;
		}
	}

	function addChoice(choice: VNChoice): void {
		pendingChoices.push(choice);
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


	function showOnomatopeia(
		params: Onomatopoeia & { wait?: boolean; mode?: TriggerMode }
	): void {
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
			activeOnomatopoeia.push(item);
		}
	}

	function handleOnomatopoeiaComplete(id: string): void {
		const index = activeOnomatopoeia.findIndex((o) => o.id === id);
		if (index !== -1) {
			const item = activeOnomatopoeia[index];
			activeOnomatopoeia.splice(index, 1);

			if (item.mode === 'mono' && onomatopoeiaQueue.length > 0) {
				const next = onomatopoeiaQueue.shift();
				if (next) {
					activeOnomatopoeia.push(next);
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

	function clear(): void {
		characters.clear();
		sceneCharacters.length = 0;
		pendingChoices.length = 0;
		messageVisible = false;
		message.character = null;
		message.text = '';
		message.wait = false;
		choicesVisible = false;
		activeOnomatopoeia.length = 0;
		onomatopoeiaQueue.length = 0;
		navigationTriggered = false;
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
				activeOnomatopoeia
			};
		},
		setCharacters,
		getCharacter,
		findInScene,
		joinScene,
		leaveScene,
		showMessage,
		hideMessage,
		setWaitResolver,
		continueFromWait,
		addChoice,
		showChoices,
		hideChoices,
		clearChoices,
		setNavigationTriggered,
		getPendingJumpTo,
		isNavigationTriggered,
		showOnomatopeia,
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
