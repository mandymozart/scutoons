<script lang="ts">
	import type { SceneCharacter as VNCharacterRef, Character } from '@scutoons/content';
	import { asset } from '$lib/utils/cdn.ts';
	import { isKnownMissing, markMissing } from '$lib/utils/missing-images.svelte.ts';
	import { DEFAULT_MOOD, legacyCharacterColor, legacyCharacterImage, legacyCharacterLabel, toonAsset } from '@scutoons/content';
	import { FULL, STAGE, type FullPlace } from '$lib/visual-novel/stage.ts';

	// A full character in the centre group (stage.ts `fullLayout`): in the middle of the screen (owner 2026-10-07; the box,
	// z 100, may cover its lower body), the speaker un-dimmed with the glow in front of the stage, the others dimmed behind it.
	let {
		charRef,
		character,
		toonId,
		isActive = false,
		place,
		ground
	}: {
		charRef: VNCharacterRef;
		character: Character;
		toonId: string;
		isActive: boolean;
		place: FullPlace;
		/** px from the viewport bottom to the feet (centred: half the free height) */
		ground: number;
	} = $props();

	let rawPath = '';
	const imagePath = $derived(() => {
		const variant = 'full';
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
		return toonAsset(toonId, path);
	});
	const imageUrl = $derived(asset(imagePath()));
</script>

<div
	class="character-full"
	class:active={isActive}
	data-depth={place.depth}
	data-key="{charRef.name}:{charRef.persona || 'regular'}"
	style:left="{place.left}px"
	style:width="{place.width}px"
	style:height="{place.height}px"
	style:bottom="{ground}px"
	style:z-index={isActive ? FULL.zSpeaker : FULL.zBehind - place.depth}
	style:filter={isActive ? `drop-shadow(0 0 0.5rem ${legacyCharacterColor(character, charRef.persona)})` : `brightness(${STAGE.dim})`}
>
	<img src={imageUrl} onerror={() => markMissing(rawPath)} alt={legacyCharacterLabel(character, charRef.persona)} />
</div>

<style>
	.character-full {
		position: fixed;
		display: flex;
		align-items: flex-end;
		justify-content: center;
		transform-origin: bottom center;
		transition:
			left 0.3s ease,
			bottom 0.3s ease,
			width 0.3s ease,
			height 0.3s ease,
			filter 0.3s ease;
		pointer-events: none;
	}

	.character-full.active {
		animation: wiggle-full 0.5s ease-in-out infinite;
	}

	.character-full img {
		width: 100%;
		height: 100%;
		object-fit: contain;
		object-position: bottom;
		display: block;
	}
</style>
