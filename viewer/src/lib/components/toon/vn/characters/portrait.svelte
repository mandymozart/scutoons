<script lang="ts">
	import type { SceneCharacter as VNCharacterRef, Character } from '@scutoons/content';
	import { asset } from '$lib/utils/cdn.ts';
	import { isKnownMissing, markMissing } from '$lib/utils/missing-images.svelte.ts';
	import { DEFAULT_MOOD, legacyCharacterColor, legacyCharacterImage, legacyCharacterLabel } from '@scutoons/content';
	import type { StagePlace } from '$lib/visual-novel/stage.ts';

	// above the dialogue box (z 100) while speaking, else below it, the front one on top
	const Z_INDEX_ACTIVE_CHARACTER = 101;
	const Z_INDEX_INACTIVE_BASE = 99;

	let {
		charRef,
		character,
		toonId,
		isActive = false,
		place
	}: {
		charRef: VNCharacterRef;
		character: Character;
		toonId: string;
		isActive: boolean;
		/** the place on the stage (stageLayout: size, right, lift, depth) */
		place: StagePlace;
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
	data-depth={place.depth}
	style:width="{place.size}px"
	style:height="{place.size}px"
	style:right="{place.right}px"
	style:bottom="calc(12rem + {place.lift}px)"
	style:z-index={isActive ? Z_INDEX_ACTIVE_CHARACTER : Z_INDEX_INACTIVE_BASE - place.depth}
	style:filter={isActive
		? `drop-shadow(0 0 0.5rem ${legacyCharacterColor(character, charRef.persona)})`
		: `brightness(${Math.min(0.9, place.brightness)})`}
>
	<img src={imageUrl} onerror={() => markMissing(rawPath)} alt={legacyCharacterLabel(character, charRef.persona)} />
</div>

<style>
	.character-portrait {
		position: fixed;
		display: flex;
		align-items: flex-end;
		justify-content: center;
		transition:
			right 0.3s ease,
			bottom 0.3s ease,
			width 0.3s ease,
			height 0.3s ease,
			filter 0.3s ease;
		pointer-events: none;
	}

	.character-portrait.active {
		animation: wiggle-portrait 0.5s ease-in-out infinite;
	}

	.character-portrait img {
		width: 100%;
		height: 100%;
		object-fit: contain;
		object-position: bottom;
		display: block;
	}
</style>
