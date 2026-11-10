<script lang="ts">
	// The phone of the handheld camera (#126, #469; the studio's PhonePairing – workflows/storyboard-studio README "Handheld
	// camera", STUDIO-IN-CREATOR §2.6): "Connect a phone" → a one-time code as a QR code to the creator's barebone phone
	// page (/h/<code>, no sign-in, 10 minutes), the link's state (WebXR 6DoF or rotation only) and the relay rung the PC
	// is on (QUIC · WebSocket), Recenter (the phone's pose
	// now = the shot camera), the live smoothing (0 = the raw handheld look), the scale (scene metres per phone metre)
	// and Record / Stop – the phone has the same buttons. Compact-inspector rows (PropRow + ScrubNumber).
	import qrcode from 'qrcode-generator';
	import { PropRow, ScrubNumber } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';
	import { openHand } from '$lib/handheld/hand-link.ts';

	let { sb }: { sb: StoryboardPage } = $props();
	const hh = $derived(sb.hh);

	const qr = $derived.by(() => {
		if (!hh.url) return '';
		const q = qrcode(0, 'M');
		q.addData(hh.url);
		q.make();
		return q.createSvgTag({ cellSize: 4, margin: 2, scalable: true });
	});
	const mode = $derived(hh.phone?.mode === 'xr' ? m['storyboard.hh_mode_xr']() : hh.phone?.mode === 'orientation' ? m['storyboard.hh_mode_orientation']() : m['storyboard.hh_mode_none']());
	// the rung is the core's word (QUIC · WebSocket), shown as it says it
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
</script>

{#if hh.status === 'idle'}
	<button type="button" class="secondary" onclick={() => hh.pair(openHand)} title={m['storyboard.hh_connect_hint']()} data-testid="hh-connect">{m['storyboard.hh_connect']()}</button>
{:else}
	{#if hh.status === 'waiting' && hh.url}
		<div class="pair">
			<div class="qr" role="img" aria-label={m['storyboard.hh_qr']({ url: hh.url })} data-testid="hh-qr">{@html qr}</div>
			<p class="sc-caption">{m['storyboard.hh_scan']()} <a href={hh.url} target="_blank" rel="noreferrer" data-testid="hh-link">{m['storyboard.hh_link']()}</a></p>
		</div>
	{/if}
	<p class="state" role="status" data-testid="hh-state">{state}</p>
	<PropRow label={m['storyboard.hh_smooth']()} hint={m['storyboard.hh_smooth_live_hint']()}>
		<ScrubNumber label={m['storyboard.hh_smooth_live']()} icon="wind" min={0} max={1} step={0.05} required value={hh.smooth} onchange={(v) => hh.setSmooth(v ?? 0)} />
	</PropRow>
	<PropRow label={m['storyboard.hh_scale']()} hint={m['storyboard.hh_scale_hint']()}>
		<ScrubNumber label={m['storyboard.hh_scale_label']()} icon="maximize-2" min={0.1} max={5} step={0.05} required value={hh.scale} onchange={(v) => hh.setScale(v ?? 1)} />
	</PropRow>
	<div class="buttons">
		<button type="button" class="secondary" onclick={() => hh.unpair()} data-testid="hh-disconnect">{m['storyboard.hh_disconnect']()}</button>
		<button type="button" class="secondary" onclick={() => hh.recenter()} disabled={hh.status !== 'connected'} title={m['storyboard.hh_recenter_hint']()} data-testid="hh-recenter">{m['storyboard.hh_recenter']()}</button>
		{#if hh.recording}
			<button type="button" class="rec" onclick={() => hh.stopRec()} data-testid="hh-stop">{m['storyboard.hh_stop']({ s: hh.recSeconds.toFixed(1) })}</button>
		{:else}
			<button type="button" onclick={() => hh.startRec()} disabled={hh.status !== 'connected' || !hh.live} title={m['storyboard.hh_record_hint']()} data-testid="hh-record">{m['storyboard.hh_record']()}</button>
		{/if}
	</div>
{/if}

<style>
	.pair {
		display: flex;
		align-items: flex-start;
		gap: var(--space-1);
	}
	.qr {
		flex: none;
		width: 9rem;
		background: var(--paper);
		line-height: 0;
	}
	.qr :global(svg) {
		display: block;
		width: 100%;
		height: auto;
	}
	.pair p {
		flex: 1;
		min-width: 0;
	}
	.pair p,
	.state {
		margin: 0;
		overflow-wrap: anywhere;
	}
	.buttons {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05);
	}
	.rec {
		background: var(--state-error);
		border-color: var(--state-error);
		color: var(--paper);
	}
</style>
