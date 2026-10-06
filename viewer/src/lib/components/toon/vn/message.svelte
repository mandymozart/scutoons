<script lang="ts">
	import { getVisualNovelContext } from '$lib/contexts/visual-novel.svelte';
	import Choices from './choices.svelte';
	import { legacyCharacterLabel } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';

	const vn = getVisualNovelContext();

	const visible = $derived(vn.state.messageVisible || vn.state.choicesVisible);
	// the characters stand on the box's top edge (vn/characters.svelte)
	let height = $state(0);
	$effect(() => vn.setBoxHeight(height));
	// … and the box's own lift above the viewport bottom (1rem, more above the draft-preview label: --vn-inset-bottom)
	let box = $state<HTMLElement | null>(null);
	$effect(() => {
		if (!box) return;
		const el = box;
		const measure = () => vn.setBoxBottom(parseFloat(getComputedStyle(el).bottom) || 0);
		measure();
		const observer = new MutationObserver(measure);
		observer.observe(document.documentElement, { attributes: true, attributeFilter: ['class', 'style'] });
		window.addEventListener('resize', measure);
		return () => {
			observer.disconnect();
			window.removeEventListener('resize', measure);
		};
	});

	// the page doesn't scroll while the story waits: interactions/vn-page.svelte (the whole page's scripts and triggers)

	// an askText (agents/EXPRESSIONS.md §6): one input line under the question; Enter / ✓ answers, Skip takes the default
	let typed = $state('');
	let input = $state<HTMLInputElement | null>(null);
	const question = $derived(vn.state.question);
	$effect(() => {
		if (!question) return;
		typed = '';
		// the reader types right away (the page doesn't scroll meanwhile)
		queueMicrotask(() => input?.focus({ preventScroll: true }));
	});
	function submit(event: SubmitEvent) {
		event.preventDefault();
		vn.answer(typed);
	}

	function handleClick() {
		if (vn.state.message.wait && !vn.state.choicesVisible) {
			vn.continueFromWait();
		}
	}
</script>

<div class="vn-message" class:visible bind:offsetHeight={height} bind:this={box}>
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
		{#if question}
			<form class="ask" onsubmit={submit} data-testid="vn-ask">
				<input
					bind:this={input}
					bind:value={typed}
					type="text"
					maxlength={question.maxLength}
					placeholder={question.placeholder}
					aria-label={m['chapter.ask_label']()}
					autocomplete="off"
					enterkeyhint="done"
					data-testid="vn-ask-input"
				/>
				<button type="submit" class="ok" aria-label={m['chapter.ask_ok']()} title={m['chapter.ask_ok']()} data-testid="vn-ask-ok">✓</button>
			</form>
			<button type="button" class="skip" onclick={() => vn.answer(null)} title={question.fallback || undefined} data-testid="vn-ask-skip">{m['chapter.ask_skip']()}</button>
		{/if}
		<Choices />
	</div>
</div>

<style>
	.vn-message {
		position: fixed;
		bottom: calc(1rem + var(--vn-inset-bottom, 0rem));
		left: 1rem;
		right: 1rem;
		min-height: 10rem;
		background: white;
		color: black;
		padding: 1rem;
		border: 2px solid black;
		transform: translateY(calc(14rem + var(--vn-inset-bottom, 0rem)));
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

	.ask {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-bottom: 0.5rem;
	}
	.ask input {
		flex: 1;
		min-width: 0;
		height: var(--control-height);
		padding: 0 0.75rem;
		border: 2px solid black;
		background: white;
		color: black;
		font: inherit;
		font-size: 1rem;
	}
	/* the round ✓ (icon-only buttons are round) */
	.ok {
		flex: 0 0 var(--control-height);
		width: var(--control-height);
		min-width: 0;
		height: var(--control-height);
		padding: 0;
		border-radius: 50%;
		border: 2px solid black;
		background: black;
		color: white;
		font-size: 1.25rem;
		line-height: 1;
		cursor: pointer;
	}
	.skip {
		background: none;
		border: 0;
		padding: 0;
		color: #555;
		font-size: 0.875rem;
		text-decoration: underline;
		cursor: pointer;
	}

	.text.narrator {
		font-style: italic;
	}
</style>
