<script lang="ts">
	// The storyboard's handheld camera phone page (owner 2026-10-20: a barebone page, reached only through the Handheld
	// panel's QR code, no sign-in – the one-time code is its right): outside the creator's frame, no paraglide
	// (phone-words.ts), no three. The screen is the one handheld's (@scutoons/ui HandheldStage); here only Record / Stop
	// and what the PC says.
	import { onDestroy, onMount } from 'svelte';
	import { page } from '$app/state';
	import { HandheldStage } from '@scutoons/ui';
	import { openHand } from '$lib/handheld/hand-link.ts';
	import { PhoneSession } from '$lib/handheld/phone-session.svelte.ts';
	import { phoneWords } from '$lib/handheld/phone-words.ts';

	const { words: w, lang } = phoneWords();
	// the page is keyed by its code (a new QR → a new page)
	// svelte-ignore state_referenced_locally
	const s = new PhoneSession(page.params.code ?? '', openHand);

	onMount(() => {
		document.documentElement.lang = lang;
		document.title = w.title;
		void s.connect();
	});
	onDestroy(() => s.close());

	const status = $derived(
		s.status === 'nocode' ? w.noCode : s.status === 'expired' ? w.expired : s.status === 'failed' ? w.failed : s.status === 'closed' ? w.closed : s.status === 'connecting' ? w.connecting : !s.on ? w.off : s.error ? w.noSensor : ''
	);
	const live = $derived(s.status === 'connected');
</script>

<svelte:head><meta name="theme-color" content="#000000" /></svelte:head>

<HandheldStage words={w} hand={s.hand} mode={s.mode} tracking={s.tracking} xr={s.xr} {live} big={s.shot ?? w.title} {status} onrecenter={s.recenter}>
	{#snippet top()}
		{#if live}<span data-testid="phone-rung">{w.connected} · {s.rung}</span>{/if}
		{#if s.recording}<span class="rec" data-testid="phone-rec">● {s.seconds.toFixed(1)} s</span>{/if}
	{/snippet}
	{#snippet actions()}
		{#if s.mode !== 'none'}
			<button type="button" class:rec-on={s.recording} disabled={!s.on} onclick={s.record} data-testid="phone-record">{s.recording ? w.stop : w.record}</button>
		{/if}
	{/snippet}
</HandheldStage>

<style>
	.rec {
		padding: 0 var(--space-05);
		background: var(--tool-stop);
		color: var(--tool-pressed-text);
	}
</style>
