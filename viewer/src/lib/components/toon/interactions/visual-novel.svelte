<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import type { Group } from '@scutoons/content';
	import type { VNEvent } from '@scutoons/content';
	import { getGameContext } from '$lib/contexts/game.svelte';
	import { getVisualNovelContext } from '$lib/contexts/visual-novel.svelte';
	import { buildJumpToUrl } from '$lib/visual-novel/jump-to.ts';
	import { interpolateVariables } from '$lib/visual-novel/utils.ts';
	import Characters from '../vn/characters.svelte';
	import Onomatopeias from '../vn/onomatopeias.svelte';
	import MessageBox from '../vn/message.svelte';

	let { group }: { group: Group } = $props();

	const game = getGameContext();
	const vn = getVisualNovelContext();
	
	let cancelled = false;

	async function handleShowMessage(event: VNEvent & { type: 'showMessage' }): Promise<void> {
		const { text, character: characterName, persona, wait = false } = event.params;

		let charRef = null;
		if (characterName) {
			const inScene = vn.findInScene(characterName, persona);
			if (!inScene) {
				console.error(
					`Character "${characterName}" is not in the scene. Add them with joinScene first.`
				);
				return;
			}
			charRef = inScene;
		}

		const interpolatedText = interpolateVariables(text, game);
		vn.showMessage(charRef, interpolatedText, wait);

		if (wait) {
			return new Promise<void>((resolve) => {
				vn.setWaitResolver(resolve);
			});
		}
	}

	function handleHideMessage(event: VNEvent & { type: 'hideMessage' }): void {
		vn.hideMessage();
	}

	async function handleShowChoices(event: VNEvent & { type: 'showChoices' }): Promise<boolean> {
		vn.showChoices();

		return new Promise((resolve) => {
			const checkChoices = () => {
				if (vn.state.pendingChoices.length === 0) {
					const shouldStopProcessing = vn.isNavigationTriggered();
					resolve(shouldStopProcessing);
				} else {
					setTimeout(checkChoices, 50);
				}
			};
			checkChoices();
		});
	}

	function handleAddChoice(event: VNEvent & { type: 'addChoice' }): void {
		const { label, onSelect } = event.params;
		const interpolatedLabel = interpolateVariables(label, game);
		vn.addChoice({ label: interpolatedLabel, onSelect });
	}

	function handleJoinScene(event: VNEvent & { type: 'joinScene' }): void {
		const { characters } = event.params;
		const charsWithDefaults = characters.map((char) => ({
			...char,
			// refs written as { character } are the same as { name }
			name: char.name ?? (char as { character?: string }).character ?? '',
			position: char.position || { variant: 'avatar' as const }
		}));
		vn.joinScene(charsWithDefaults);
	}

	function handleLeaveScene(event: VNEvent & { type: 'leaveScene' }): void {
		const { characters } = event.params;
		vn.leaveScene(characters);
	}

	async function handleShowOnomatopoeia(event: VNEvent & { type: 'showOnomatopeia' }): Promise<void> {
		const { wait = false } = event.params;

		vn.showOnomatopeia(event.params);

		if (wait) {
			return new Promise<void>((resolve) => {
				vn.setOnomatopoeiaResolver(resolve);
			});
		}
	}

	async function handleJumpTo(event: VNEvent & { type: 'jumpTo' }): Promise<void> {
		const { target, wait = false } = event.params;
		
		const destinationUri = buildJumpToUrl(target, game.state.toonId);
		
		if (!destinationUri) {
			console.warn('Invalid jumpTo target', target);
			return;
		}
		
		if (wait) {
			await new Promise<void>((resolve) => {
				vn.setWaitResolver(resolve);
			});
		}
		
		goto(destinationUri);
	}

	async function processEvent(event: VNEvent): Promise<boolean> {
		switch (event.type) {
			case 'showMessage':
				await handleShowMessage(event);
				return false;

			case 'hideMessage':
				handleHideMessage(event);
				return false;

			case 'showChoices':
				return await handleShowChoices(event);

			case 'addChoice':
				handleAddChoice(event);
				return false;

			case 'joinScene':
				handleJoinScene(event);
				return false;

			case 'leaveScene':
				handleLeaveScene(event);
				return false;

			case 'showOnomatopeia':
				await handleShowOnomatopoeia(event);
				return false;

			case 'jumpTo':
				await handleJumpTo(event);
				return true;

			default:
				console.warn(`Unknown event type: ${(event as any).type}`);
				return false;
		}
	}

	async function runEvents(): Promise<void> {
		const events = group.events || [];

		for (let i = 0; i < events.length; i++) {
			if (cancelled) return;
			

			const event = events[i];
			console.log(
				`Event ${i + 1}/${events.length}: ${event.type}`,
				event.params
			);

			await processEvent(event);
		}
	}

	onMount(() => {
		cancelled = false;
		runEvents();
	});
	
	onDestroy(() => {
		cancelled = true;
		vn.clear();
	});
</script>

<Characters />
<Onomatopeias />
<MessageBox />

<style>
</style>
