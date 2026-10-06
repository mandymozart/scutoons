<script lang="ts">
	import { getVisualNovelContext } from '$lib/contexts/visual-novel.svelte';
	import Choices from './choices.svelte';
	import { legacyCharacterLabel } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';

	const vn = getVisualNovelContext();

	const visible = $derived(vn.state.messageVisible || vn.state.choicesVisible);
	// the characters stand on the box's top edge while it shows (vn/characters.svelte); hidden it slides away – 0, so they
	// stand at the page's bottom (owner 2026-10-06, BACKLOG M.3: offsetHeight ignores the slide-out)
	let height = $state(0);
	$effect(() => vn.setBoxHeight(visible ? height : 0));

	// the page doesn't scroll while the story waits: interactions/vn-page.svelte (the whole page's scripts and triggers)

	// an askText (agents/EXPRESSIONS.md §6): one input line under the question; Enter / ✓ answers, the answer button
	// (its own text) answers with the default – or nothing
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
			<!-- the ready answer as an action button with its own text (owner 2026-10-06: no "Skip" link; button text and
			     stored value are two fields); no text: no button – an empty ✓ goes on (the default or nothing) -->
			{#if question.answer}
				<button type="button" class="vn-choice answer" data-control-exempt="dialogue answer (may wrap)" onclick={() => vn.answer(null)} data-testid="vn-ask-skip">{question.answer}</button>
			{/if}
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
	/* the answer button looks like a dialogue choice (choices.svelte) */
	.answer {
		display: block;
		width: 100%;
		background: white;
		color: black;
		padding: 1rem 1.5rem; /* control-ok: a dialogue answer may wrap */
		border: 2px solid lightgrey;
		cursor: pointer;
		font: inherit;
		font-size: 1rem;
		text-align: left;
	}
	.answer:hover {
		border-color: #333;
	}
	.answer:active {
		border-color: black;
	}

	.text.narrator {
		font-style: italic;
	}
</style>
