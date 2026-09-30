<script lang="ts">
	import type { SceneCharacter as VNCharacterRef, Character } from '@scutoons/content';
	import { asset } from '$lib/utils/cdn.ts';
	import { isKnownMissing, markMissing } from '$lib/utils/missing-images.svelte.ts';
	import { DEFAULT_MOOD, legacyCharacterColor, legacyCharacterImage, legacyCharacterLabel } from '@scutoons/content';
	import { STAGE, type StagePlace } from '$lib/visual-novel/stage.ts';

	// A character on the stage, left (`avatar` view) or right (`portrait` view), standing on the dialogue box (z 100):
	// the speaker in front of the box with the glow, everyone else behind it (dimmed), the front one on top.
	const Z_INDEX_ACTIVE_CHARACTER = 101;
	const Z_INDEX_INACTIVE_BASE = 90;

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
		/** the place on the stage (stageLayout: side, size, offset, depth) */
		place: StagePlace;
		/** px from the viewport bottom to the feet (the box top) */
		ground: number;
	} = $props();

	let rawPath = '';
	const imagePath = $derived(() => {
		const variant = charRef.position?.variant === 'avatar' ? 'avatar' : 'portrait';
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
	class:framed={charRef.position?.variant === 'avatar'}
	style:background-color={charRef.position?.variant === 'avatar' ? legacyCharacterColor(character, charRef.persona) : undefined}
	data-depth={place.depth}
	data-side={place.side}
	data-key="{charRef.name}:{charRef.persona || 'regular'}"
	style:width="{place.size}px"
	style:height="{place.size}px"
	style:left={place.side === 'left' ? `${place.offset}px` : 'auto'}
	style:right={place.side === 'right' ? `${place.offset}px` : 'auto'}
	style:bottom="{ground}px"
	style:z-index={isActive ? Z_INDEX_ACTIVE_CHARACTER : Z_INDEX_INACTIVE_BASE - place.depth}
	style:filter={isActive
		? `drop-shadow(0 0 0.5rem ${legacyCharacterColor(character, charRef.persona)})`
		: `brightness(${STAGE.dim})`}
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
			left 0.3s ease,
			bottom 0.3s ease,
			width 0.3s ease,
			height 0.3s ease,
			filter 0.3s ease;
		pointer-events: none;
	}

	.character-portrait.active {
		animation: wiggle-portrait 0.5s ease-in-out infinite;
	}

	/* the avatar view keeps its frame: persona colour behind the head, 2px ink border, radius-sm */
	.character-portrait.framed {
		border: 2px solid black;
		border-radius: 0.5rem;
		overflow: hidden;
		box-sizing: border-box;
	}

	.character-portrait.framed img {
		object-fit: cover;
	}

	.character-portrait img {
		width: 100%;
		height: 100%;
		object-fit: contain;
		object-position: bottom;
		display: block;
	}
</style>
