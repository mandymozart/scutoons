<script lang="ts">
	import { getVisualNovelContext } from '$lib/contexts/visual-novel.svelte';
	import Choices from './choices.svelte';

	const vn = getVisualNovelContext();

	const visible = $derived(vn.state.messageVisible || vn.state.choicesVisible);

	function handleClick() {
		if (vn.state.message.wait && !vn.state.choicesVisible) {
			vn.continueFromWait();
		}
	}
</script>

<div
	class="vn-message"
	class:visible
	class:clickable={vn.state.message.wait && !vn.state.choicesVisible}
	onclick={handleClick}
	role={vn.state.message.wait && !vn.state.choicesVisible ? 'link' : undefined}
	tabindex={vn.state.message.wait && !vn.state.choicesVisible ? 0 : undefined}
	onkeydown={(e) => e.key === 'Enter' && handleClick()}
	aria-label={vn.state.message.wait && !vn.state.choicesVisible ? 'Continue' : undefined}
>
	{#if vn.state.message.character}
		{@const character = vn.getCharacter(vn.state.message.character.name)}
		{#if character}
			<div class="character-name">{character.label}</div>
		{/if}
	{/if}
	{#if vn.state.message.text}
		<div class="text" class:narrator={!vn.state.message.character}>
			{vn.state.message.text}
		</div>
	{/if}
	<Choices />
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
		z-index: 2;
	}

	.vn-message.visible {
		transform: translateY(0);
		pointer-events: auto;
	}

	.vn-message.clickable {
		cursor: pointer;
	}

	.vn-message.clickable::after {
		content: '...';
		position: absolute;
		bottom: 0.5rem;
		right: 1rem;
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
