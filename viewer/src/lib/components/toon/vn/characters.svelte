<script lang="ts">
	import { getVisualNovelContext } from '$lib/contexts/visual-novel.svelte';
	import { page } from '$app/stores';
	import CharacterAvatar from './character-avatar.svelte';
	import CharacterPortrait from './character-portrait.svelte';
	import CharacterFull from './character-full.svelte';

	const vn = getVisualNovelContext();
	const toonId = $derived($page.params.toonId || '');

	const sceneCharacters = $derived(vn.state.sceneCharacters);
	const activeSpeaker = $derived(vn.state.message.character?.name);

	const avatarCharacters = $derived(
		sceneCharacters.filter((c) => c.position?.variant === 'avatar')
	);
	const portraitCharacters = $derived(
		sceneCharacters.filter((c) => c.position?.variant === 'portrait')
	);
	const fullCharacters = $derived(sceneCharacters.filter((c) => c.position?.variant === 'full'));

	/**
	 * Creates an ordered array of character names with speaker first (position 0 = furthest left)
	 * @param chars - Array of character references
	 * @returns Ordered array where index = stack position
	 */
	function getStackedOrder(chars: typeof sceneCharacters): string[] {
		const names = chars.map((c) => c.name);
		
		// If there's an active speaker, move them to the front
		if (activeSpeaker && names.includes(activeSpeaker)) {
			return [activeSpeaker, ...names.filter((n) => n !== activeSpeaker)];
		}
		
		return names;
	}

	const avatarStackOrder = $derived(getStackedOrder(avatarCharacters));
	const portraitStackOrder = $derived(getStackedOrder(portraitCharacters));

	function getStackPosition(charName: string, stackOrder: string[]): number {
		return stackOrder.indexOf(charName);
	}
</script>

{#if avatarCharacters.length > 0}
	{#each avatarCharacters as charRef}
		{@const character = vn.getCharacter(charRef.name)}
		{#if character}
			<CharacterAvatar
				{charRef}
				{character}
				{toonId}
				isActive={charRef.name === activeSpeaker}
				stackPosition={getStackPosition(charRef.name, avatarStackOrder)}
			/>
		{/if}
	{/each}
{/if}

{#if portraitCharacters.length > 0}
	<div class="portraits-slot">
		{#each portraitCharacters as charRef}
			{@const character = vn.getCharacter(charRef.name)}
			{#if character}
				<CharacterPortrait
					{charRef}
					{character}
					{toonId}
					isActive={charRef.name === activeSpeaker}
					stackPosition={getStackPosition(charRef.name, portraitStackOrder)}
				/>
			{/if}
		{/each}
	</div>
{/if}

<!-- Full characters centered (breaking 4th wall) -->
{#if fullCharacters.length > 0}
	<div class="full-slot">
		{#each fullCharacters as charRef}
			{@const character = vn.getCharacter(charRef.name)}
			{#if character}
				<CharacterFull {charRef} {character} {toonId} isActive={charRef.name === activeSpeaker} />
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
