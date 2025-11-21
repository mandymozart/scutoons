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
		const variant = 'avatar';
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
	class="character-avatar"
	class:active={isActive}
	style:transform="translateX({stackOrder * 20}px)"
	style:z-index={100 - stackOrder}
	style:background-color={character.color}
>
	<img src={imageUrl} alt={character.label} />
</div>

<style>
	.character-avatar {
		width: 3rem;
		height: 3rem;
		border-radius: .5rem;
		overflow: hidden;
		border: 2px solid black;
		background: white;
		transition: all 0.3s ease;
		position: relative;
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
