<script lang="ts">
	import { getVisualNovelContext, sceneKey } from '$lib/contexts/visual-novel.svelte';
	import { page } from '$app/stores';
	import { stageLayout } from '$lib/visual-novel/stage.ts';
	import CharacterPortrait from './characters/portrait.svelte';
	import CharacterFull from './characters/full.svelte';

	// VN layout (DESIGN.md → "VN layout"): the view of the joinScene ref is the slot. `avatar` = the left slot in the
	// dialogue box while speaking (message.svelte), `portrait` = the stage on the right (front = the speaker, going back =
	// smaller, higher, further right), `full` = centred.
	const vn = getVisualNovelContext();
	const toonId = $derived($page.params.toonId || '');

	let width = $state(375);
	let height = $state(812);
	let rem = $state(16);
	$effect(() => {
		// the root font size is fluid (14 / 16 / 24 px), re-read it when the viewport changes
		void width;
		rem = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
	});

	const sceneCharacters = $derived(vn.state.sceneCharacters);
	/** the speaking character + persona (scene key) */
	const activeSpeaker = $derived(vn.state.message.character ? sceneKey(vn.state.message.character) : undefined);

	const portraitCharacters = $derived(sceneCharacters.filter((c) => c.position?.variant === 'portrait'));
	const fullCharacters = $derived(sceneCharacters.filter((c) => c.position?.variant === 'full'));

	/** front to back: the speaker first, then in the order they joined */
	const portraitOrder = $derived.by(() => {
		const keys = portraitCharacters.map((c) => sceneKey(c));
		return activeSpeaker && keys.includes(activeSpeaker) ? [activeSpeaker, ...keys.filter((k) => k !== activeSpeaker)] : keys;
	});
	const places = $derived(stageLayout(portraitCharacters.length, width, height, rem));
</script>

<svelte:window bind:innerWidth={width} bind:innerHeight={height} />

{#each portraitCharacters as charRef (sceneKey(charRef))}
	{@const character = vn.getCharacter(charRef.name)}
	{@const place = places[portraitOrder.indexOf(sceneKey(charRef))]}
	{#if character && place}
		<CharacterPortrait {charRef} {character} {toonId} {place} isActive={sceneKey(charRef) === activeSpeaker} />
	{/if}
{/each}

<!-- Full characters centered (breaking 4th wall) -->
{#if fullCharacters.length > 0}
	<div class="full-slot">
		{#each fullCharacters as charRef (sceneKey(charRef))}
			{@const character = vn.getCharacter(charRef.name)}
			{#if character}
				<CharacterFull {charRef} {character} {toonId} isActive={sceneKey(charRef) === activeSpeaker} />
			{/if}
		{/each}
	</div>
{/if}

<style>
	.full-slot {
		position: fixed;
		inset: 0;
		z-index: 1;
	}
</style>
