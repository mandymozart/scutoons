<script lang="ts">
	import type { VNCharacterRef, Character } from '@scutoons/content';
	import { asset } from '$lib/utils/cdn.ts';
	import { DEFAULT_MOOD, legacyCharacterImage, legacyCharacterLabel } from '@scutoons/content';

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

	const imagePath = $derived(() => {
		const variant = 'avatar';

		const mood = charRef.mood || DEFAULT_MOOD;
		// moods: MOODS of @scutoons/content (neutral = the fallback)
		// a look transition image (shown briefly), else the fallback rule (resolveExpression in @scutoons/content)
		const path = charRef.transition || legacyCharacterImage(character, { persona: charRef.persona, look: charRef.look, mood }, variant);

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
	class="character-avatar"
	class:active={isActive}
	style:left="{stackPosition * 0.5 + 1.5}rem"
	style:z-index={isActive ? Z_INDEX_ACTIVE_CHARACTER : Z_INDEX_INACTIVE_BASE - stackPosition}
	style:background-color={character.color}
>
	<img src={imageUrl} alt={legacyCharacterLabel(character, charRef.persona)} />
</div>

<style>
	.character-avatar {
		position: fixed;
		bottom: 13rem;
		left: 1.5rem;
		width: 3rem;
		height: 3rem;
		border-radius: 0.5rem;
		overflow: hidden;
		border: 2px solid black;
		background: white;
		transition: all 0.3s ease;
	}

	.character-avatar.active {
		border-color: red;
		animation: wiggle-avatar 0.5s ease-in-out infinite;
	}

	.character-avatar img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
</style>
