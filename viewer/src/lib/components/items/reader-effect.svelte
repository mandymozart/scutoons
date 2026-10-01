<script lang="ts">
	import { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A running reader effect (ITEMS.md "Substances"): `psychedelic` lets the slices drift in hue and wobble gently
	 * (`html[data-effect]`, CSS below) – off with "prefers-reduced-motion" – with a bar to end it at once. Game effect only.
	 */
	const inventory = getInventoryContext();
	const running = $derived(inventory.effects.find((e) => e.kind === 'psychedelic'));
	let now = $state(Date.now());
	$effect(() => {
		const t = setInterval(() => (now = Date.now()), 5_000);
		return () => clearInterval(t);
	});
	const minutes = $derived(running ? Math.max(1, Math.ceil((Date.parse(running.until) - now) / 60_000)) : 0);
	$effect(() => {
		const root = document.documentElement;
		if (running) root.dataset.effect = running.kind;
		else delete root.dataset.effect;
		return () => delete root.dataset.effect;
	});
</script>

{#if running}
	<div class="effect-bar" role="status" data-testid="reader-effect" data-kind={running.kind}>
		<span>{m['items.effect_psychedelic']()} · {m['items.effect_left']({ min: minutes })}</span>
		<button type="button" onclick={() => inventory.endEffect(running.kind)} data-testid="reader-effect-end">{m['items.effect_end']()}</button>
	</div>
{/if}

<style>
	.effect-bar {
		position: fixed;
		left: 0.5rem;
		top: 3rem;
		z-index: 999;
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.25rem 0.25rem 0.25rem 0.75rem;
		background: var(--paper, #fff);
		color: var(--ink, #000);
		border: 2px solid var(--ink, #000);
		font-size: 0.75rem;
	}
	.effect-bar button {
		padding: 0.25rem 0.75rem;
	}
	@media (prefers-reduced-motion: no-preference) {
		:global(html[data-effect='psychedelic'] [data-slice-index] img) {
			animation:
				psychedelic-hue 9s linear infinite,
				psychedelic-wobble 7s ease-in-out infinite;
		}
	}
	@keyframes -global-psychedelic-hue {
		from {
			filter: hue-rotate(0deg) saturate(1.2);
		}
		to {
			filter: hue-rotate(360deg) saturate(1.2);
		}
	}
	@keyframes -global-psychedelic-wobble {
		0%,
		100% {
			transform: none;
		}
		25% {
			transform: skewX(0.6deg) scale(1.005);
		}
		75% {
			transform: skewX(-0.6deg) scale(1.005);
		}
	}
</style>
