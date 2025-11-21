import { setContext, getContext } from 'svelte';
import type { Character, VNChoice, Message, VNCharacterRef } from '$lib/models/visual-novel.ts';

const VN_KEY = Symbol('visual-novel');


export type VisualNovelContext = ReturnType<typeof createVisualNovelContext>;

export function createVisualNovelContext() {
	const characters = $state(new Map<string, Character>());
	const sceneCharacters = $state<VNCharacterRef[]>([]);
	const pendingChoices = $state<VNChoice[]>([]);
	const isProcessingEvents = $state(false);

	let messageVisible = $state(false);
	const message = $state<Message>({
		character: null,
		text: '',
		wait: false
	});
	let choicesVisible = $state(false);
	let waitResolver: (() => void) | null = null;

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

	function joinScene(chars: VNCharacterRef[]): void {
		for (const newChar of chars) {
			const existingIndex = sceneCharacters.findIndex((c) => c.name === newChar.name);

			if (existingIndex !== -1) {
				const existing = sceneCharacters[existingIndex];
				const existingVariant = existing.position?.variant;
				const newVariant = newChar.position?.variant;
				const existingMood = existing.mood;
				const newMood = newChar.mood;

				if (existingVariant !== newVariant || existingMood !== newMood) {
					console.log(
						`Character "${newChar.name}" already in scene (variant: "${existingVariant}", mood: "${existingMood}"), replacing with (variant: "${newVariant}", mood: "${newMood}")`
					);
					sceneCharacters.splice(existingIndex, 1);
					sceneCharacters.push(newChar);
				} else {
					console.log(
						`Character "${newChar.name}" already in scene with same variant "${existingVariant}" and mood "${existingMood}", skipping`
					);
				}
			} else {
				sceneCharacters.push(newChar);
			}
		}
	}

	function leaveScene(chars: VNCharacterRef[]): void {
		const namesToRemove = chars.map((c) => c.name);
		const filtered = sceneCharacters.filter((c) => !namesToRemove.includes(c.name));
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

	function clear(): void {
		characters.clear();
		sceneCharacters.length = 0;
		pendingChoices.length = 0;
		messageVisible = false;
		message.character = null;
		message.text = '';
		message.wait = false;
		choicesVisible = false;
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
				choicesVisible
			};
		},
		setCharacters,
		getCharacter,
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
		clear
	};
}

export function setVisualNovelContext() {
	const context = createVisualNovelContext();
	setContext(VN_KEY, context);
	return context;
}

export function getVisualNovelContext() {
	return getContext<VisualNovelContext>(VN_KEY);
}
