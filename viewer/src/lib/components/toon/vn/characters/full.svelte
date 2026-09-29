<script lang="ts">
	import type { VNCharacterRef, Character } from '@scutoons/content';
	import { asset } from '$lib/utils/cdn.ts';
	import { DEFAULT_MOOD } from '@scutoons/content';

	let {
		charRef,
		character,
		toonId,
		isActive = false
	}: {
		charRef: VNCharacterRef;
		character: Character;
		toonId: string;
		isActive: boolean;
	} = $props();

	const imagePath = $derived(() => {
		const variant = 'full';
		const mood = charRef.mood || DEFAULT_MOOD;

		let path =
			character.images?.[mood]?.[variant] ||
			character.images?.[DEFAULT_MOOD]?.[variant] ||
			character.image;

		if (!path) {
			console.error(
				`No image found for character "${character.name}" (mood: ${mood}, variant: ${variant})`
			);
			return '';
		}

		// a leading / = from the CDN root (a registry character whose images live outside this toon's folder)
		return path.startsWith('/') ? path : `${toonId}/${path}`;
	});
	const imageUrl = $derived(asset(imagePath()));
</script>

<div class="character-full" class:active={isActive}>
	<img src={imageUrl} alt={character.label} />
</div>

<style>
	.character-full {
		position: fixed;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 5;
		transition: all 0.3s ease;
	}

	.character-full.active {
		animation: wiggle-full 0.5s ease-in-out infinite;
	}

	.character-full img {
		max-height: 90vh;
		max-width: 90vw;
		object-fit: contain;
		display: block;
	}
</style>
