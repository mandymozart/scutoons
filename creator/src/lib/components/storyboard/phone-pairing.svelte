<script lang="ts">
	// The phone of the handheld camera (#126, #469; lap 2 C3b): "Connect a phone" → a one-time code as a QR code to the
	// creator's barebone phone page (/h/<code>, no sign-in, 10 minutes) – the conditions (Chrome, the same Wi-Fi) are the
	// code's hover text –, the phone's link to copy, the link's state with the relay rung the PC is on (the core's word:
	// QUIC · WebSocket), the live smoothing and the scale, then Disconnect · Recenter · Record (the one primary action; the
	// phone has the same buttons).
	import { QrCode } from '@scutoons/ui';
	import { Button } from '@scutoons/ui/editor';
	import { m } from '$lib/paraglide/messages.js';
	import { toast } from '$lib/toasts.ts';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';
	import { openFollow, openHand } from '$lib/handheld/hand-link.ts';

	let { sb }: { sb: StoryboardPage } = $props();
	const hh = $derived(sb.hh);

	const mode = $derived(hh.phone?.mode === 'xr' ? m['storyboard.hh_mode_xr']() : hh.phone?.mode === 'orientation' ? m['storyboard.hh_mode_orientation']() : m['storyboard.hh_mode_none']());
	const rung = $derived(hh.rung || m['storyboard.hh_joining']());
	const state = $derived.by(() => {
		switch (hh.status) {
			case 'waiting':
				return `${m['storyboard.hh_waiting']()} · ${rung}`;
			case 'connected':
				return `${m['storyboard.hh_connected']()} · ${mode}${hh.phone?.tracking === false ? ` · ${m['storyboard.hh_tracking_lost']()}` : ''} · ${rung}`;
			case 'closed':
				return `${m['storyboard.hh_closed']()} · ${rung}`;
			case 'failed':
				return m['storyboard.hh_failed']({ reason: hh.error ?? '' });
		}
		return '';
	});

	async function copyLink() {
		if (!hh.url) return;
		try {
			await navigator.clipboard.writeText(hh.url);
			toast(m['storyboard.hh_link_copied']());
		} catch {
			window.open(hh.url, '_blank', 'noreferrer');
		}
	}
</script>

{#if hh.status === 'idle'}
	<Button icon="smartphone" title={m['storyboard.hh_connect_hint']()} onclick={() => hh.pair(openHand, openFollow)} data-testid="hh-connect">{m['storyboard.hh_connect']()}</Button>
{:else}
	<div class="pair">
		{#if hh.status === 'waiting' && hh.url}
			<span class="code" title={m['storyboard.hh_scan']()}><QrCode text={hh.url} label={m['storyboard.hh_qr']({ url: hh.url })} testid="hh-qr" /></span>
		{/if}
		<span class="words">
			{#if hh.status === 'waiting' && hh.url}
				<span>{m['storyboard.hh_scan_short']()}</span>
				<Button variant="quiet" icon="copy" title={m['storyboard.hh_link_hint']()} onclick={copyLink} data-testid="hh-link">{m['storyboard.hh_link']()}</Button>
			{/if}
			<span class="state" class:on={hh.status === 'connected'} class:off={hh.status === 'failed'} role="status" data-testid="hh-state"><span class="dot"></span>{state}</span>
		</span>
	</div>
	<div class="ranges">
		<label class="sc-label" for="hh-smooth" title={m['storyboard.hh_smooth_live_hint']()}>{m['storyboard.hh_smooth']()}</label>
		<input id="hh-smooth" type="range" min="0" max="1" step="0.05" value={hh.smooth} aria-label={m['storyboard.hh_smooth_live']()} title="{m['storyboard.hh_smooth_live']()}: {hh.smooth.toFixed(2)}" oninput={(e) => hh.setSmooth(Number(e.currentTarget.value))} data-testid="hh-smooth" />
		<label class="sc-label" for="hh-scale" title={m['storyboard.hh_scale_hint']()}>{m['storyboard.hh_scale']()}</label>
		<input id="hh-scale" type="range" min="0.1" max="5" step="0.05" value={hh.scale} aria-label={m['storyboard.hh_scale_label']()} title="{m['storyboard.hh_scale_label']()}: {hh.scale.toFixed(2)}" oninput={(e) => hh.setScale(Number(e.currentTarget.value))} data-testid="hh-scale" />
	</div>
	<div class="buttons">
		<Button onclick={() => hh.unpair()} data-testid="hh-disconnect">{m['storyboard.hh_disconnect']()}</Button>
		<Button onclick={() => hh.recenter()} disabled={hh.status !== 'connected'} title={m['storyboard.hh_recenter_hint']()} data-testid="hh-recenter">{m['storyboard.hh_recenter']()}</Button>
		<span class="grow"></span>
		{#if hh.recording}
			<Button variant="primary" destroy onclick={() => hh.stopRec()} data-testid="hh-stop">{m['storyboard.hh_stop']({ s: hh.recSeconds.toFixed(1) })}</Button>
		{:else}
			<Button variant="primary" onclick={() => hh.startRec()} disabled={hh.status !== 'connected' || !hh.live} title={m['storyboard.hh_record_hint']()} data-testid="hh-record">{m['storyboard.hh_record']()}</Button>
		{/if}
	</div>
{/if}

<style>
	.pair {
		display: flex;
		align-items: flex-start;
		gap: 0.8rem;
	}
	.code {
		flex: none;
		width: 7rem;
	}
	.code :global(.qr) {
		width: 100%;
		border-radius: 0.3rem;
		overflow: hidden;
	}
	.words {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 0.45rem;
		min-width: 0;
	}
	.state {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		color: var(--tool-muted);
		font-size: var(--editor-small);
		overflow-wrap: anywhere;
	}
	.dot {
		flex: none;
		width: 0.45rem;
		height: 0.45rem;
		border: var(--editor-edge) solid currentColor;
		border-radius: 50%;
		box-sizing: border-box;
	}
	.state.on {
		color: var(--tool-text);
	}
	.state.on .dot {
		background: var(--tool-go);
		border-color: var(--tool-go);
	}
	.state.off .dot {
		background: var(--tool-stop);
		border-color: var(--tool-stop);
	}
	.ranges {
		display: grid;
		grid-template-columns: 4.5rem 1fr;
		gap: 0.45rem 0.6rem;
		align-items: center;
		margin-top: 0.7rem;
	}
	.ranges input {
		width: 100%;
	}
	.buttons {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		margin-top: 0.7rem;
	}
	.grow {
		flex: 1;
	}
</style>
