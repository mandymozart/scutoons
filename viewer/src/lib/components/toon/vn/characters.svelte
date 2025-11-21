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
	const fullCharacters = $derived(
		sceneCharacters.filter((c) => c.position?.variant === 'full')
	);

	function getStackOrder(charName: string, chars: typeof sceneCharacters): number {
		if (charName === activeSpeaker) return 0;
		const index = chars.findIndex((c) => c.name === charName);
		const speakerIndex = chars.findIndex((c) => c.name === activeSpeaker);
		if (speakerIndex === -1) return index;
		return index > speakerIndex ? index - 1 : index;
	}
</script>


	{#if avatarCharacters.length > 0}
		<div class="avatars-slot">
			{#each avatarCharacters as charRef}
				{@const character = vn.getCharacter(charRef.name)}
				{#if character}
					<CharacterAvatar
						{charRef}
						{character}
						{toonId}
						isActive={charRef.name === activeSpeaker}
						stackOrder={getStackOrder(charRef.name, avatarCharacters)}
					/>
				{/if}
			{/each}
		</div>
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
						stackOrder={getStackOrder(charRef.name, portraitCharacters)}
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
				<CharacterFull
					{charRef}
					{character}
					{toonId}
					isActive={charRef.name === activeSpeaker}
				/>
			{/if}
		{/each}
	</div>
	{/if}

<style>

	.avatars-slot {
		position: fixed;
		bottom: 12rem;
		left: 1rem;
		display: flex;
		align-items: center;
		gap: 0;
		z-index: 3;
	}

	.full-slot {
		position: fixed;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 2rem;
		padding: 1rem;
		padding-bottom: 13rem;
		z-index: 1;
	}

	.portraits-slot {
		position: fixed;
		inset: 0;
		display: flex;
		align-items: flex-end;
		justify-content: center;
		gap: 2rem;
		padding: 1rem;
		padding-bottom: 13rem;
		z-index: 1;
	}
</style>
