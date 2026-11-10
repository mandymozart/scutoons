<script lang="ts">
	// The handheld camera's phone page (#126; owner 2026-10-20: "the phone camera … yes, but a barebone page so it doesnt
	// need to load so much grap. it is only accessed through a qr code when linked anyhow" – DECISIONS, STUDIO-IN-CREATOR
	// §2.6): reached only through the Handheld panel's QR code, no sign-in (the one-time code is its right), without the
	// creator's frame (it lives outside the (app) route group), no paraglide (phone-words.ts), no three. It connects, asks for
	// the motion sensor (iOS: a tap), streams the phone's pose to the PC through the relay session (the live road: QUIC first, the WebSocket last; #469) and has three controls:
	// Record / Stop, Recenter (the phone's pose now = the shot camera) – the PC shows the shot and records.
	import { onDestroy, onMount } from 'svelte';
	import { page } from '$app/state';
	import { openHand } from '$lib/handheld/hand-link.ts';
	import { PhoneSession } from '$lib/handheld/phone-session.svelte.ts';
	import { phoneWords } from '$lib/handheld/phone-words.ts';

	const { words: w, lang } = phoneWords();
	// the page is keyed by its code (a new QR → a new page)
	// svelte-ignore state_referenced_locally
	const s = new PhoneSession(page.params.code ?? '', openHand);
	let root = $state<HTMLElement>();

	onMount(() => {
		document.documentElement.lang = lang;
		document.title = w.title;
		void s.connect();
	});
	onDestroy(() => s.close());

	const status = $derived(
		s.status === 'nocode' ? w.noCode : s.status === 'expired' ? w.expired : s.status === 'failed' ? w.failed : s.status === 'closed' ? w.closed : s.status === 'connecting' ? w.connecting : !s.on ? w.off : s.error ? w.noSensor : ''
	);
	const mode = $derived(s.mode === 'xr' ? (s.tracking ? w.modeXr : w.trackingLost) : s.mode === 'orientation' ? w.modeOrientation : '');
	const live = $derived(s.status === 'connected');
</script>

<svelte:head><meta name="theme-color" content="#000000" /></svelte:head>

<main class="phone sc-tool" bind:this={root} data-testid="handheld-phone">
	<div class="stage" role="application" aria-label={w.stage} onpointerdown={s.down} onpointermove={s.move} onpointerup={s.up} onpointercancel={s.up}>
		<p class="big">{s.shot ?? w.title}</p>
		{#if mode}<p class="small">{mode}</p>{/if}
		{#if s.mode === 'orientation'}<p class="small">{w.dragHint}</p>{/if}
		{#if s.mode === 'none' && live}<p class="small">{w.startHint}</p>{/if}
	</div>
	<header class="bar">
		<span>{w.title}</span>
		{#if live}<span data-testid="phone-rung">{w.connected} · {s.rung}</span>{/if}
		{#if s.recording}<span class="rec" data-testid="phone-rec">● {s.seconds.toFixed(1)} s</span>{/if}
	</header>
	{#if status}<p class="status" role="status" data-testid="phone-status">{status}</p>{/if}
	{#if live}
		<footer class="bar bottom">
			{#if s.mode === 'none'}
				{#if s.xr}<button type="button" onclick={() => root && s.startXR(root)} data-testid="phone-start-xr">{w.startAr}</button>{/if}
				<button type="button" class:secondary={s.xr} onclick={() => s.startOrientation()} data-testid="phone-start">{s.xr ? w.rotationOnly : w.start}</button>
			{:else}
				<button type="button" class="secondary" title={w.recenterHint} onclick={s.recenter} data-testid="phone-recenter">{w.recenter}</button>
				<button type="button" class:rec-on={s.recording} disabled={!s.on} onclick={s.record} data-testid="phone-record">{s.recording ? w.stop : w.record}</button>
			{/if}
		</footer>
	{/if}
</main>

<style>
	:global(body) {
		margin: 0;
		background: var(--ink);
	}
	.phone {
		position: fixed;
		inset: 0;
		background: var(--ink);
		color: var(--paper);
		overflow: hidden;
		touch-action: none;
		user-select: none;
		font-family: var(--font-family-mono, monospace);
	}
	.stage {
		position: absolute;
		inset: 0;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: var(--space-05);
		padding: var(--space-1);
		text-align: center;
	}
	.stage p {
		margin: 0;
	}
	.big {
		font-size: var(--type-title-size, 1.5rem);
	}
	.small {
		font-size: var(--type-caption-size);
		opacity: 0.8;
	}
	.bar {
		position: absolute;
		left: 0;
		right: 0;
		top: 0;
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05);
		align-items: center;
		padding: var(--space-05) var(--space-1);
		font-size: var(--type-caption-size);
		pointer-events: none;
	}
	.bottom {
		top: auto;
		bottom: 0;
		justify-content: center;
		padding-bottom: calc(var(--space-1) + env(safe-area-inset-bottom));
		pointer-events: auto;
	}
	/* on ink: the primary is paper, the secondary an outline (the studio's phone page) */
	.bottom button {
		flex: 1 1 0;
		max-width: 12rem;
		background: var(--paper);
		color: var(--ink);
		border-color: var(--paper);
	}
	.bottom button.secondary {
		background: var(--ink);
		color: var(--paper);
	}
	.rec,
	.bottom button.rec-on {
		background: var(--state-error);
		border-color: var(--state-error);
		color: var(--paper);
	}
	.rec {
		padding: 0 var(--space-05);
	}
	.status {
		position: absolute;
		left: var(--space-1);
		right: var(--space-1);
		top: 30%;
		margin: 0;
		text-align: center;
	}
</style>
