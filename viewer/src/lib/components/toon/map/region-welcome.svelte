<script lang="ts">
	import type { ReaderRegion } from '@scutoons/content';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The welcome after a `migrate` (agents/MAP.md §8.10, owner 2026-10-12: "the Ortseingangsschild of the new region is
	 * shown and the elected representative of that region in portrait, the sign in the background … if there is no
	 * character elected, a generic narrator says a region-specific greeting"): the sign fills the screen behind, the
	 * representative's portrait stands in front with the greeting in a VN-like text box – without one, the narrator's box
	 * alone. A tap, Enter, Space or Esc goes on.
	 */
	let {
		region,
		representative,
		ondone
	}: {
		region: ReaderRegion;
		/** the elected representative (name + portrait path), or null: the narrator speaks */
		representative: { name: string; portrait: string; color?: string } | null;
		ondone: () => void;
	} = $props();

	const greeting = $derived(region.greeting || m['map.region_welcome']({ name: region.name }));
	let button = $state<HTMLButtonElement>();
	$effect(() => button?.focus());
	function onKey(e: KeyboardEvent) {
		if (['Enter', ' ', 'Escape'].includes(e.key)) {
			e.preventDefault();
			ondone();
		}
	}
</script>

<svelte:window onkeydown={onKey} />

<div class="welcome" role="dialog" aria-modal="true" aria-label={region.name} data-testid="region-welcome">
	{#if region.sign?.src}
		<img class="sign" src={asset(region.sign.src)} alt={region.claim ? `${region.name} – ${region.claim}` : region.name} />
	{:else}
		<p class="sign-text">{region.name}{#if region.claim}<small>{region.claim}</small>{/if}</p>
	{/if}
	<button bind:this={button} class="stage" type="button" onclick={ondone} aria-label={m['map.region_continue']()}>
		{#if representative}
			<img class="portrait" src={asset(representative.portrait)} alt="" style:--rep-color={representative.color ?? null} data-testid="region-representative" />
		{/if}
		<span class="box" class:narrator={!representative}>
			{#if representative}<span class="speaker">{representative.name}</span>{/if}
			<span class="text">{greeting}</span>
		</span>
	</button>
</div>

<style>
	.welcome {
		position: fixed;
		inset: 0;
		z-index: 1200;
		background: var(--ink, #000);
		display: flex;
		align-items: center;
		justify-content: center;
		animation: fade 0.4s ease-out both;
	}
	/* the sign in the background, the whole board visible */
	.sign {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		object-fit: contain;
		background: var(--paper, #fff);
	}
	.sign-text {
		color: var(--paper, #fff);
		font-family: var(--font-display, inherit);
		font-size: 2.5rem;
		text-align: center;
		small {
			display: block;
			font-size: 1rem;
		}
	}
	/* the stage over it: the portrait at the bottom left, the text box beside / under it – one tap target */
	.stage {
		position: absolute;
		inset: auto 0 0 0;
		display: flex;
		align-items: flex-end;
		gap: 0.75rem;
		padding: 1rem;
		background: none;
		border: none;
		cursor: pointer;
		text-align: left;
	}
	.portrait {
		width: min(40vw, 14rem);
		max-height: 60vh;
		object-fit: contain;
		filter: drop-shadow(0 0.25rem 0.75rem rgb(0 0 0 / 0.4));
		animation: rise 0.5s ease-out 0.2s both;
	}
	.box {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		background: var(--paper, #fff);
		border: 0.125rem solid var(--ink, #000);
		padding: 0.75rem 1rem;
		font-size: 1rem;
		line-height: 1.4;
		color: var(--ink, #000);
		animation: rise 0.4s ease-out 0.35s both;
	}
	.box.narrator .text {
		font-style: italic;
	}
	.speaker {
		font-weight: 700;
		text-transform: uppercase;
		font-size: 0.75rem;
		letter-spacing: 0.05em;
	}
	@keyframes fade {
		from {
			opacity: 0;
		}
	}
	@keyframes rise {
		from {
			opacity: 0;
			transform: translateY(1rem);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.welcome,
		.portrait,
		.box {
			animation: none;
		}
	}
</style>
