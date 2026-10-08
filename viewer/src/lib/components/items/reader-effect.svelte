<script lang="ts">
	import { READER_EFFECTS, type ReaderEffectKind } from '@scutoons/content';
	import { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { setAudioEffect } from '$lib/audio/audio-effects.ts';
	import PlasmaOverlay from './plasma-overlay.svelte';
	import HazeOverlay from './haze-overlay.svelte';

	/**
	 * A running reader effect (ITEMS.md "Substances"; each item chooses its own – catalog `effect.kind`, @scutoons/content
	 * READER_EFFECTS, owner 2026-10-08: "different drugs hit differently"); an info line says how long it lasts – it can't
	 * be ended (owner: "no button to exit"), eating something that sobers (catalog `sober`) shortens it. Game effect only.
	 * - `psychedelic` ("Farbrausch", LSD …): a moving plasma cloud blended over everything (plasma-overlay.svelte; the
	 *   frames don't tilt – owner: "tilting is not good"), the sound through a sweeping high-pass and an echo
	 * - `haze` ("Dunst", weed): drifting haze over the screen (haze-overlay.svelte), the sound muffled
	 * The sound: audio/audio-effects.ts over everything heard. One at a time – the one started last.
	 */
	const inventory = getInventoryContext();
	const running = $derived(inventory.effects.findLast((e) => (READER_EFFECTS as readonly string[]).includes(e.kind)) as { kind: ReaderEffectKind; until: string } | undefined);
	const LABEL: Record<ReaderEffectKind, () => string> = { psychedelic: () => m['items.effect_psychedelic'](), haze: () => m['items.effect_haze']() };
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
	// the sound of it: on while it runs, faded out when it ends
	$effect(() => {
		setAudioEffect(running?.kind ?? null);
		return () => setAudioEffect(null);
	});
</script>

<!-- something worn tints everything (catalog worn_tint – sunglasses: orange; owner 2026-10-08), under any effect -->
{#if inventory.wornTint}
	<div class="tint" style:background={inventory.wornTint} aria-hidden="true" data-testid="worn-tint"></div>
{/if}

{#if running}
	{#if running.kind === 'psychedelic'}<PlasmaOverlay />{:else if running.kind === 'haze'}<HazeOverlay />{/if}
	<div class="effect-bar" role="status" data-testid="reader-effect" data-kind={running.kind}>
		<span>{LABEL[running.kind]()} · {m['items.effect_left']({ min: minutes })}</span>
	</div>
{/if}

<style>
	:global(:root) {
		--sc-tint-blend: multiply;
		--sc-tint-opacity: 0.6;
	}
	.tint {
		position: fixed;
		inset: 0;
		z-index: 989;
		pointer-events: none;
		mix-blend-mode: var(--sc-tint-blend);
		opacity: var(--sc-tint-opacity);
		animation: tint-in 0.6s ease-out both;
	}
	@keyframes tint-in {
		from {
			opacity: 0;
		}
	}
	.effect-bar {
		position: fixed;
		left: 0.5rem;
		top: 3rem;
		z-index: 999;
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.25rem 0.75rem;
		background: var(--paper, #fff);
		color: var(--ink, #000);
		border: 2px solid var(--ink, #000);
		font-size: 0.75rem;
	}

</style>
