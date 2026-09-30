<script lang="ts">
	import type { SceneCharacter as VNCharacterRef, Character } from '@scutoons/content';
	import { asset } from '$lib/utils/cdn.ts';
	import { isKnownMissing, markMissing } from '$lib/utils/missing-images.svelte.ts';
	import { DEFAULT_MOOD, legacyCharacterColor, legacyCharacterImage, legacyCharacterLabel } from '@scutoons/content';

		const Z_INDEX_MESSAGE_BOX = 100;
	const Z_INDEX_ACTIVE_CHARACTER = 101;
	const Z_INDEX_INACTIVE_BASE = 99;

	let {
		charRef,
		character,
		toonId,
		isActive = false,
		stackPosition = 0
	}: {
		charRef: VNCharacterRef;
		character: Character;
		toonId: string;
		isActive: boolean;
		stackPosition: number;
	} = $props();

	let rawPath = '';
	const imagePath = $derived(() => {
		const variant = 'portrait';
		const mood = charRef.mood || DEFAULT_MOOD;

		// a look transition image (shown briefly), else the fallback rule (resolveExpression in @scutoons/content)
		const path = charRef.transition || legacyCharacterImage(character, { persona: charRef.persona, look: charRef.look, mood }, variant, { isMissing: isKnownMissing });

		rawPath = path;
		if (!path) {
			console.error(
				`No image found for character "${character.name}" (look: ${charRef.look ?? 'default'}, mood: ${mood}, variant: ${variant})`
			);
			return '';
		}

		// a leading / = from the CDN root (a registry character whose images live outside this toon's folder)
		return path.startsWith('/') ? path : `${toonId}/${path}`;
	});
	const imageUrl = $derived(asset(imagePath()));
</script>

<div
	class="character-portrait"
	class:active={isActive}
style:right="{stackPosition * 2 + 1.5}rem"
	style:z-index={isActive ? Z_INDEX_ACTIVE_CHARACTER : Z_INDEX_INACTIVE_BASE - stackPosition}
	style:filter={isActive ? `drop-shadow(0 0 0.5rem ${legacyCharacterColor(character, charRef.persona)})` : ''}
>
	<img src={imageUrl} onerror={() => markMissing(rawPath)} alt={legacyCharacterLabel(character, charRef.persona)} />
	{#if charRef.mood}
		<span class="mood-badge">{charRef.mood}</span>
	{/if}
</div>

<style>
	.character-portrait {
		position: fixed;
		bottom: 12rem;
		max-height: 80vh;
		max-width: 40vw;
		display: flex;
		align-items: flex-end;
		transition: all 0.3s ease;
		filter: brightness(0.9);
	}

	.character-portrait.active {
		filter: brightness(1);
		animation: wiggle-portrait 0.5s ease-in-out infinite;
	}

	.character-portrait img {
		max-height: 100%;
		max-width: 100%;
		object-fit: contain;
		display: block;
	}
</style>
