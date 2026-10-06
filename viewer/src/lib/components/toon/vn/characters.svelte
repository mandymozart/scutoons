<script lang="ts">
	import { getVisualNovelContext, sceneKey } from '$lib/contexts/visual-novel.svelte';
	import { page } from '$app/stores';
	import { fullLayout, STAGE, stageLayout } from '$lib/visual-novel/stage.ts';
	import CharacterPortrait from './characters/portrait.svelte';
	import CharacterFull from './characters/full.svelte';

	// VN layout (DESIGN.md → "VN layout"): everyone in the scene is visible, standing on the dialogue box. The view of the
	// joinScene ref is the side: `avatar` = the left group, `portrait` = the right group, `full` = the centre group (feet on
	// the box's bottom edge). In each group the speaker is in front, the others by recency of speaking; placement in
	// stage.ts.
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

	/**
	 * front to back: the speaker, then by recency of speaking (who just stopped speaking right behind the speaker), then
	 * the ones who haven't spoken since they joined, in join order
	 */
	const frontToBack = (keys: string[]) => {
		const spoke = vn.state.lastSpoke;
		const rest = keys.filter((k) => k !== activeSpeaker);
		const spoken = rest.filter((k) => spoke[k]).sort((a, b) => spoke[b] - spoke[a]);
		const silent = rest.filter((k) => !spoke[k]);
		return [...(activeSpeaker && keys.includes(activeSpeaker) ? [activeSpeaker] : []), ...spoken, ...silent];
	};
	const leftCharacters = $derived(sceneCharacters.filter((c) => c.position?.variant === 'avatar'));
	const rightCharacters = $derived(sceneCharacters.filter((c) => c.position?.variant === 'portrait'));
	const fullCharacters = $derived(sceneCharacters.filter((c) => c.position?.variant === 'full'));
	const leftOrder = $derived(frontToBack(leftCharacters.map(sceneKey)));
	const rightOrder = $derived(frontToBack(rightCharacters.map(sceneKey)));
	const fullOrder = $derived(frontToBack(fullCharacters.map(sceneKey)));
	const groups = $derived(stageLayout(leftCharacters.length, rightCharacters.length, width, rem));
	const fullPlaces = $derived(fullLayout(fullCharacters.length, width, height, rem));
	/** the full characters' feet: the box's bottom edge (the box sits 1 rem above the viewport bottom) */
	const fullGround = $derived(Math.round(rem));
	/** feet on the shown box's top (the box sits 1 rem above the bottom), a few px into it; without a box on the
	 *  follow-up bar's top; with neither at the page's bottom like the full characters (owner 2026-10-06, BACKLOG M.3) */
	const ground = $derived(
		Math.round(vn.state.boxHeight > 0 ? rem + vn.state.boxHeight - STAGE.sink : vn.state.endHeight > 0 ? vn.state.endHeight - STAGE.sink : rem)
	);
	const onStage = $derived([
		...leftCharacters.map((c) => ({ c, place: groups.left.places[leftOrder.indexOf(sceneKey(c))] })),
		...rightCharacters.map((c) => ({ c, place: groups.right.places[rightOrder.indexOf(sceneKey(c))] }))
	]);
</script>

<svelte:window bind:innerWidth={width} bind:innerHeight={height} />

{#each onStage as { c: charRef, place } (sceneKey(charRef))}
	{@const character = vn.getCharacter(charRef.name)}
	{#if character && place}
		<CharacterPortrait
			{charRef}
			{character}
			{toonId}
			{place}
			ground={sceneKey(charRef) === activeSpeaker ? ground : ground - STAGE.behind * rem}
			isActive={sceneKey(charRef) === activeSpeaker}
		/>
	{/if}
{/each}

<!-- Full characters: the centre group (breaking the 4th wall), under the box -->
{#each fullCharacters as charRef (sceneKey(charRef))}
	{@const character = vn.getCharacter(charRef.name)}
	{@const place = fullPlaces[fullOrder.indexOf(sceneKey(charRef))]}
	{#if character && place}
		<CharacterFull {charRef} {character} {toonId} {place} ground={fullGround} isActive={sceneKey(charRef) === activeSpeaker} />
	{/if}
{/each}
