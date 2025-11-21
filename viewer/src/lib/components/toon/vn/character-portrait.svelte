<script lang="ts">
	import type { VNCharacterRef, Character } from '$lib/models/visual-novel.ts';
	import { asset } from '$lib/utils/cdn.ts';

	let {
		charRef,
		character,
		toonId,
		isActive = false,
		stackOrder = 0
	}: {
		charRef: VNCharacterRef;
		character: Character;
		toonId: string;
		isActive: boolean;
		stackOrder: number;
	} = $props();

	const imagePath = $derived(() => {
		const variant = 'portrait';
		const mood = charRef.mood || 'default';

		let path =
			character.images?.[mood]?.[variant] ||
			character.images?.['default']?.[variant] ||
			character.image;

		if (!path) {
			console.error(
				`No image found for character "${character.name}" (mood: ${mood}, variant: ${variant})`
			);
			return '';
		}

		return `${toonId}/${path}`;
	});
	const imageUrl = $derived(asset(imagePath()));
</script>

<div
	class="character-portrait"
	class:active={isActive}
	style:transform="translateX({stackOrder * 30}px)"
	style:z-index={50 - stackOrder}
>
	<img src={imageUrl} alt={character.label} />
	{#if charRef.mood}
		<span class="mood-badge">{charRef.mood}</span>
	{/if}
</div>

<style>
	.character-portrait {
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
