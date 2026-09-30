<script lang="ts">
	import { getVisualNovelContext } from '$lib/contexts/visual-novel.svelte';
	import Choices from './choices.svelte';
	import { legacyCharacterLabel } from '@scutoons/content';

	const vn = getVisualNovelContext();

	const visible = $derived(vn.state.messageVisible || vn.state.choicesVisible);
	// the characters stand on the box's top edge (vn/characters.svelte)
	let height = $state(0);
	$effect(() => vn.setBoxHeight(height));

	function handleClick() {
		if (vn.state.message.wait && !vn.state.choicesVisible) {
			vn.continueFromWait();
		}
	}
</script>

<div class="vn-message" class:visible bind:offsetHeight={height}>
	{#if vn.state.message.wait && !vn.state.choicesVisible}
		<button class="continue-overlay" onclick={handleClick} aria-label="Continue">
			<span class="sr-only">Continue</span>
		</button>
	{/if}
	<div class="body">
		{#if vn.state.message.character}
			{@const character = vn.getCharacter(vn.state.message.character.name)}
			{#if character}
				<div class="character-name">{legacyCharacterLabel(character, vn.state.message.character.persona)}</div>
			{/if}
		{/if}
		{#if vn.state.message.text}
			<div class="text" class:narrator={!vn.state.message.character}>
				{vn.state.message.text}
			</div>
		{/if}
		<Choices />
	</div>
</div>

<style>
	.vn-message {
		position: fixed;
		bottom: 1rem;
		left: 1rem;
		right: 1rem;
		min-height: 10rem;
		background: white;
		color: black;
		padding: 1rem;
		border: 2px solid black;
		transform: translateY(14rem);
		pointer-events: none;
		transition: transform 0.1s ease;
		z-index: 100;
	}

	.vn-message.visible {
		transform: translateY(0);
		pointer-events: auto;
	}


	.continue-overlay {
		position: absolute;
		inset: 0;
		background: transparent;
		border: none;
		cursor: pointer;
		padding: 0;
		margin: 0;
		width: 100%;
		height: 100%;
	}

	.continue-overlay::after {
		content: '...';
		position: absolute;
		bottom: 0.5rem;
		right: 1rem;
		color: black;
		animation: pulse 1s ease-in-out infinite;
	}

	@keyframes pulse {
		0%,
		100% {
			opacity: 0.5;
		}
		50% {
			opacity: 1;
		}
	}

	.sr-only {
		position: absolute;
		width: 1px;
		height: 1px;
		padding: 0;
		margin: -1px;
		overflow: hidden;
		clip: rect(0, 0, 0, 0);
		white-space: nowrap;
		border-width: 0;
	}

	.character-name {
		margin-bottom: 0.5rem;
		font-size: 1rem;
		text-transform: uppercase;
	}

	.text {
		margin-bottom: .5rem;
	}

	.text.narrator {
		font-style: italic;
	}
</style>
