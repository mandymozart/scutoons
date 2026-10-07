import { setContext, getContext, type Snippet } from 'svelte';
import type { Character, VNChoice, VNEvent, Message, VNCharacterRef, SceneCharacter } from '@scutoons/content';
import { LEGACY_CHARACTER_IDS, REGULAR_PERSONA, legacyTransition, normalizeSceneRef } from '@scutoons/content';
import type { ShowTextBoxParams } from '@scutoons/content';
import type { TriggerMode } from '@scutoons/content';
import type { ActiveTextBox } from '@scutoons/reader';
import type { JumpToTarget } from '@scutoons/content';
import { createBox, createBoxData, type BoxQuestion } from '$lib/visual-novel/box-state.ts';
import { jumpProblem as jumpProblemAt, type JumpPlace } from '$lib/visual-novel/jump-to.ts';

const VN_KEY = Symbol('visual-novel');

/** How long a look transition image is shown before the new look (ms). */
const TRANSITION_MS = 800;

/** A character in a scene is identified by id + persona (Cat Pope may meet Cat Pope · Biker). */
export const sceneKey = (ref: Pick<SceneCharacter, 'name' | 'persona'>) => `${ref.name}:${ref.persona || REGULAR_PERSONA}`;

/**
 * A showTextBox event playing: its text box is drawn by its slice (`dialogPlaying`). `scope` = the group whose script /
 * slice events revealed it (linked groups share a page: a box is looked up in its own group).
 */
export type VNActiveTextBox = ActiveTextBox & { scope: string };

/** A choice on the box, with the label of the script that added it (a random jump never picks that group) */
export type VNPageChoice = VNChoice & { from?: string | null };

/**
 * An askText's input line in the box (agents/EXPRESSIONS.md §6): the page waits until it is answered. `answer(null)` =
 * skipped (the runner stores the default).
 */
export type VNQuestion = BoxQuestion;

/** text boxes are kept per group of the page: `scope` + the box id */
const boxKey = (scope: string, id: string) => `${scope}|${id}`;

export type VisualNovelContext = ReturnType<typeof createVisualNovelContext>;

let _currentContext: VisualNovelContext | undefined;

export function createVisualNovelContext() {
	const characters = $state(new Map<string, Character>());
	const sceneCharacters = $state<SceneCharacter[]>([]);
	const isProcessingEvents = $state(false);
	const activeTextBoxes = $state<VNActiveTextBox[]>([]);
	const textBoxQueue = $state<VNActiveTextBox[]>([]);
	/**
	 * Text boxes that showTextBox events reveal (`params.dialog`, agents/DESIGN.md "Onomatopoeia → text box"): the ids a
	 * group script refers to (hidden until revealed; slice events are known by their slice) and the ids already revealed
	 * on this page view (they stay).
	 */
	const dialogRefs = $state<Record<string, true>>({});
	const revealedDialogs = $state<Record<string, true>>({});
	/**
	 * The box's waiting state (owner 2026-10-06: no dead ends – lib/visual-novel/box-state.ts, pure and tested in node):
	 * the message and its click, choices, an askText question, slice-trigger runs taking turns, the page's scripts. Its
	 * reactive part lives here (`$state`), the logic there.
	 */
	const boxData = $state(createBoxData<SceneCharacter, VNPageChoice>());
	const box = createBox(boxData, { textBoxWaiting: () => activeTextBoxes.some((o) => o.wait) || textBoxQueue.some((o) => o.wait) });
	/** the page that owns the context now (a page going away clears only its own state) */
	let page = 0;
	/** where the page is (its toon, chapter, group labels): a jump into a group that isn't there stays (jump-to.ts) */
	let place: JumpPlace | null = null;
	/** the dialogue box's height (px) while it shows, else 0: the characters stand on its top edge */
	let boxHeight = $state(0);
	/**
	 * the chapter's end actions (chapter/next-bar.svelte, BACKLOG 10.48 / UI-CONCEPTS E.5): drawn as choices inside the
	 * message box (vn/message.svelte) while set – the box shows for them, the characters stand on it
	 */
	let end = $state.raw<{ render: Snippet } | null>(null);
	/** scene key → when it last spoke (a counter), for the stage order: the speaker, then by recency (vn/characters) */
	const lastSpoke = $state<Record<string, number>>({});
	let spokeCounter = 0;
	/**
	 * whoever waits for the waiting text boxes to finish (a page script and a slice-trigger run may both wait: each one is
	 * kept – one would otherwise write over the other's and hang, owner 2026-10-06: no dead ends)
	 */
	const textBoxResolvers: (() => void)[] = [];
	/**
	 * The page's group script and its label (CUTOVER 10.27): a slice event of type divert runs the script from a named
	 * point; a random jump never picks the group the reader is on. A choice's divert waits here for the script.
	 */
	let script: VNEvent[] = [];
	let scriptLabel: string | null = null;

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
		if (charRef) lastSpoke[sceneKey(charRef)] = ++spokeCounter;
		box.showMessage(charRef, text, wait);
	}

	function setScript(events: VNEvent[] | undefined, label: string | null): void {
		script = events ?? [];
		scriptLabel = label;
	}
	function setNavigationTriggered(): void {
		box.setNavigationTriggered();
	}

	/** where the page is – set by the chapter page (its toon, chapter number, group labels) */
	function setPlace(next: JumpPlace | null): void {
		place = next;
	}
	/** why a jump can't go where it says (a group label missing in this chapter) – null: it can (jump-to.ts) */
	function jumpProblem(target: JumpToTarget): string | null {
		return jumpProblemAt(target, place);
	}


	/** Reveals the event's text box (`params.dialog`); an event without one shows nothing (the validation flags it). */
	function showTextBox(params: ShowTextBoxParams, scope = ''): boolean {
		if (typeof params?.dialog !== 'string' || !params.dialog) {
			console.warn('showTextBox without params.dialog – nothing to reveal');
			return false;
		}
		const id = `box-${Date.now()}-${Math.random()}`;
		const { wait = false, mode = 'poly' as TriggerMode, ...data } = params;

		const item: VNActiveTextBox = {
			...data,
			id,
			scope,
			wait,
			mode,
			onComplete: () => handleTextBoxComplete(id)
		};

		const hasActiveMono = activeTextBoxes.some((o) => o.mode === 'mono');
		if (mode === 'mono' && hasActiveMono) {
			textBoxQueue.push(item);
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
	function activate(item: VNActiveTextBox): void {
		activeTextBoxes.push(item);
		revealedDialogs[boxKey(item.scope, item.dialog)] = true;
		const time = (item.delay ?? 0) + (item.duration ?? 2000);
		setTimeout(() => handleTextBoxComplete(item.id), Math.min(Math.max(time, 0), 10_000) + 1500);
	}

	/** a group script's text box refs (hidden until an event reveals them), per group (`scope`) */
	function registerDialogRefs(ids: string[], scope = ''): void {
		for (const id of ids) dialogRefs[boxKey(scope, id)] = true;
	}

	/** a text box waits for its showTextBox event (`sliceRefs` = the refs of its own slice's events; `scope` = its group) */
	function dialogHidden(id: string | undefined, sliceRefs?: ReadonlySet<string>, scope = ''): boolean {
		if (!id || revealedDialogs[boxKey(scope, id)]) return false;
		return !!dialogRefs[boxKey(scope, id)] || !!sliceRefs?.has(id);
	}

	/** the showTextBox event revealing this text box (of group `scope`) right now */
	function dialogPlaying(id: string | undefined, scope = ''): VNActiveTextBox | undefined {
		return id ? activeTextBoxes.find((o) => o.dialog === id && o.scope === scope) : undefined;
	}

	/**
	 * A reader page takes the context: the scene starts fresh (the page before may only go away after this one came –
	 * "Nochmal", another chain); returns its token for `pageState` / `releasePage`
	 */
	function claimPage(): number {
		page++;
		clear();
		return page;
	}
	/** the page's script line changed (stale pages are ignored) */
	function pageState(token: number, state: { busy: number; done: boolean }): void {
		if (token !== page) return;
		box.pageState(state);
	}
	/** the page went away: the scene is cleared – unless another page took the context meanwhile */
	function releasePage(token: number): void {
		if (token !== page) return;
		clear();
	}

	function handleTextBoxComplete(id: string): void {
		const index = activeTextBoxes.findIndex((o) => o.id === id);
		if (index !== -1) {
			const item = activeTextBoxes[index];
			activeTextBoxes.splice(index, 1);
			// `stay: false`: the box disappears again after its effect (hidden like before the event)
			if (item.stay === false) delete revealedDialogs[boxKey(item.scope, item.dialog)];

			if (item.mode === 'mono' && textBoxQueue.length > 0) {
				const next = textBoxQueue.shift();
				if (next) {
					activate(next);
				}
			}

			const hasWaitingItems = activeTextBoxes.some((o) => o.wait) || 
				textBoxQueue.some((o) => o.wait);
			
			if (!hasWaitingItems && textBoxResolvers.length) {
				for (const resolve of textBoxResolvers.splice(0)) resolve();
			}
		}
	}

	function setTextBoxResolver(resolver: () => void): void {
		// nothing waits any more (the box completed before the resolver came): go on at once
		if (!activeTextBoxes.some((o) => o.wait) && !textBoxQueue.some((o) => o.wait)) return resolver();
		textBoxResolvers.push(resolver);
	}

	function continueFromTextBox(): void {
		activeTextBoxes.forEach((item) => item.onComplete());
		textBoxQueue.length = 0;
	}

	/**
	 * Resets the scene when a VN group ends. The characters stay: the toon layout loads them once (setCharacters), and the
	 * next VN group (another label or chapter of the same toon) needs them again.
	 */
	function clear(): void {
		sceneCharacters.length = 0;
		for (const key of Object.keys(lastSpoke)) delete lastSpoke[key];
		activeTextBoxes.length = 0;
		textBoxQueue.length = 0;
		// a script waiting for a text box goes on (its page is gone – it is cancelled there)
		for (const resolve of textBoxResolvers.splice(0)) resolve();
		for (const key of Object.keys(dialogRefs)) delete dialogRefs[key];
		for (const key of Object.keys(revealedDialogs)) delete revealedDialogs[key];
		box.reset();
	}

	return {
		get state() {
			return {
				characters,
				sceneCharacters,
				pendingChoices: boxData.pendingChoices,
				isProcessingEvents,
				messageVisible: boxData.messageVisible,
				message: boxData.message,
				choicesVisible: boxData.choicesVisible,
				question: boxData.question,
				activeTextBoxes,
				boxHeight,
				end: end?.render ?? null,
				lastSpoke,
				eventsDone: boxData.eventsDone,
				pageBusy: boxData.pageBusy,
				pageDone: boxData.pageDone,
				runs: boxData.runs,
				/**
				 * The story waits (owner 2026-10-03 / 2026-10-06): something to answer (a message's click, choices, a question,
				 * a text box's tap) or a script / slice trigger of the page in progress – the page doesn't scroll until all of
				 * it is through (box-state.ts: in a settled state there is always something to answer then).
				 */
				frozen: box.frozen(),
				/** something on screen moves the story on (box-state.ts) */
				answerable: box.answerable()
			};
		},
		setCharacters,
		getCharacter,
		setBoxHeight: (h: number) => {
			boxHeight = h;
		},
		/** show the end actions in the box (null: none); returns a release that clears only this showing */
		setEnd: (render: Snippet | null) => {
			const mine = render ? { render } : null;
			end = mine;
			return () => {
				if (end === mine) end = null;
			};
		},
		findInScene,
		joinScene,
		leaveScene,
		showMessage,
		hideMessage: box.hideMessage,
		waitForClick: box.waitForClick,
		continueFromWait: box.continueFromWait,
		waitForContinue: box.waitForContinue,
		beginEvents: box.beginEvents,
		endOfEvents: box.endOfEvents,
		addChoice: (choice: VNPageChoice) => box.addChoice(choice),
		ask: box.ask,
		answer: box.answer,
		dropUnshownChoices: box.dropUnshownChoices,
		showChoices: box.showChoices,
		whenChoicesClosed: box.whenChoicesClosed,
		hideChoices: box.hideChoices,
		clearChoices: box.clearChoices,
		acquireRun: box.acquireRun,
		holds: box.holds,
		whenNoRun: box.whenNoRun,
		runHolds: box.runHolds,
		interrupt: box.interrupt,
		endRun: box.endRun,
		/** the box changed (the page's script queue wakes up) */
		subscribe: box.subscribe,
		idle: box.idle,
		setNavigationTriggered,
		setPlace,
		jumpProblem,
		setScript,
		get script() {
			return script;
		},
		get scriptLabel() {
			return scriptLabel;
		},
		setPendingDivert: box.setPendingDivert,
		takePendingDivert: box.takePendingDivert,
		isNavigationTriggered: box.isNavigationTriggered,
		showTextBox,
		registerDialogRefs,
		dialogHidden,
		dialogPlaying,
		setTextBoxResolver,
		continueFromTextBox,
		claimPage,
		pageState,
		releasePage,
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
