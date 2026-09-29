<script lang="ts">
	import { getVisualNovelContext, sceneKey } from '$lib/contexts/visual-novel.svelte';
	import { page } from '$app/stores';
	import CharacterAvatar from './characters/avatar.svelte';
	import CharacterPortrait from './characters/portrait.svelte';
	import CharacterFull from './characters/full.svelte';

	const vn = getVisualNovelContext();
	const toonId = $derived($page.params.toonId || '');

	const sceneCharacters = $derived(vn.state.sceneCharacters);
	/** the speaking character + persona (scene key) */
	const activeSpeaker = $derived(vn.state.message.character ? sceneKey(vn.state.message.character) : undefined);

	const avatarCharacters = $derived(
		sceneCharacters.filter((c) => c.position?.variant === 'avatar')
	);
	const portraitCharacters = $derived(
		sceneCharacters.filter((c) => c.position?.variant === 'portrait')
	);
	const fullCharacters = $derived(sceneCharacters.filter((c) => c.position?.variant === 'full'));

	function getStackedOrder(chars: typeof sceneCharacters): string[] {
		const names = chars.map((c) => sceneKey(c));
		
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
				isActive={sceneKey(charRef) === activeSpeaker}
				stackPosition={getStackPosition(sceneKey(charRef), avatarStackOrder)}
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
					isActive={sceneKey(charRef) === activeSpeaker}
					stackPosition={getStackPosition(sceneKey(charRef), portraitStackOrder)}
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
