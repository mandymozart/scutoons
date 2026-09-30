<script lang="ts">
	import type { SceneCharacter as VNCharacterRef, Character } from '@scutoons/content';
	import { asset } from '$lib/utils/cdn.ts';
	import { isKnownMissing, markMissing } from '$lib/utils/missing-images.svelte.ts';
	import { DEFAULT_MOOD, legacyCharacterColor, legacyCharacterImage, legacyCharacterLabel } from '@scutoons/content';

	// the left slot of the dialogue box (message.svelte) – placed by its parent
	let {
		charRef,
		character,
		toonId,
		isActive = false,
		size = 4
	}: {
		charRef: VNCharacterRef;
		character: Character;
		toonId: string;
		isActive: boolean;
		/** rem */
		size?: number;
	} = $props();

	let rawPath = '';
	const imagePath = $derived(() => {
		const variant = 'avatar';

		const mood = charRef.mood || DEFAULT_MOOD;
		// moods: MOODS of @scutoons/content (neutral = the fallback)
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
	class="character-avatar"
	class:active={isActive}
	style:width="{size}rem"
	style:height="{size}rem"
	style:background-color={legacyCharacterColor(character, charRef.persona)}
	style:filter={isActive ? `drop-shadow(0 0 0.5rem ${legacyCharacterColor(character, charRef.persona)})` : ''}
>
	<img src={imageUrl} onerror={() => markMissing(rawPath)} alt={legacyCharacterLabel(character, charRef.persona)} />
</div>

<style>
	.character-avatar {
		flex: none;
		border-radius: 0.5rem;
		overflow: hidden;
		border: 2px solid black;
		background: white;
		transition: filter 0.3s ease;
	}

	.character-avatar.active {
		animation: wiggle-avatar 0.5s ease-in-out infinite;
	}

	.character-avatar img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
</style>
