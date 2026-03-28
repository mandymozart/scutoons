<script lang="ts">
	import type { VNCharacterRef, Character } from '$lib/models/visual-novel.ts';
	import { asset } from '$lib/utils/cdn.ts';

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

		const mood = charRef.mood || 'neutral';
		// neutral, happy, sad, angry, mad, surprised, excited, touched, tease and disgust
		let path =
			character.images?.[mood]?.[variant] ||
			character.images?.['neutral']?.[variant] ||
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
	style:left="{stackPosition * 0.5 + 1.5}rem"
	style:z-index={isActive ? Z_INDEX_ACTIVE_CHARACTER : Z_INDEX_INACTIVE_BASE - stackPosition}
	style:background-color={character.color}
>
	<img src={imageUrl} alt={character.label} />
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
