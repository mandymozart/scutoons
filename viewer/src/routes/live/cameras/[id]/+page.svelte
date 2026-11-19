<script lang="ts">
	// /live/cameras/<id>?live=<code> (research/LIVE-CAMERAS.md §2, §3): one camera of a live session, reached by its QR.
	// The phone asks first (naming a take-over), then holds the camera and steers it with its motion – the one handheld's
	// screen (@scutoons/ui HandheldStage); Leave lets it go, a newer claim shows the warning and Take it back. A drop or a
	// reload takes the camera again by itself while nobody else claimed it (the core's rule).
	import { onDestroy } from 'svelte';
	import { page } from '$app/state';
	import { HandheldStage } from '@scutoons/ui';
	import CameraAsk from '$lib/components/camera/camera-ask.svelte';
	import { LIVE_QUIC, LIVE_URL } from '$lib/config.ts';
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { CameraSession } from '$lib/live-camera.svelte.ts';
	import { cameraHref, cameraName, listHref } from '$lib/live-cameras.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { sessionOf } from '$lib/reader-session.ts';

	const id = page.params.id ?? '';
	const relay = sessionOf(page.url.searchParams, LIVE_URL, LIVE_QUIC);
	const session = getSessionContext();
	const s = relay ? new CameraSession(relay, id, session.isLoggedIn) : null;
	onDestroy(() => s?.close());

	const w = {
		title: m['live_cameras.camera'](),
		start: m['live_cameras.start'](),
		startAr: m['live_cameras.start_ar'](),
		rotationOnly: m['live_cameras.rotation_only'](),
		startHint: m['live_cameras.start_hint'](),
		recenter: m['live_cameras.recenter'](),
		recenterHint: m['live_cameras.recenter_hint'](),
		modeXr: m['live_cameras.mode_xr'](),
		modeOrientation: m['live_cameras.mode_orientation'](),
		trackingLost: m['live_cameras.tracking_lost'](),
		dragHint: m['live_cameras.drag_hint'](),
		stage: m['live_cameras.stage']()
	};
	// a forced switch: the page follows the camera the core says it holds
	const shown = $derived(s?.holding.hold === 'held' && s.holding.camera ? s.holding.camera : id);
	const cam = $derived(s?.cameras.find((c) => c.id === shown));
	const held = $derived(s?.holding.hold === 'held');
	const status = $derived(!s ? m['live_cameras.no_session']() : s.status === 'joining' ? m['live_cameras.connecting']() : s.status === 'offline' ? m['live_cameras.offline']() : held && s.denied ? m['live_cameras.no_sensor']() : '');
	const flashed = $derived(typeof s?.flash?.['label'] === 'string' ? (s.flash['label'] as string) : null);
	$effect(() => {
		if (shown !== id) history.replaceState(history.state, '', cameraHref(shown, page.url.searchParams));
	});
</script>

<svelte:head><title>{m['live_cameras.camera']()} · {cam ? cameraName(cam) : id}</title><meta name="theme-color" content="#000000" /></svelte:head>

{#if s}
	<HandheldStage words={w} hand={s.hand} mode={held ? s.mode : 'none'} tracking={s.tracking} xr={s.xr} live={held && s.status === 'connected'} big={flashed ?? (cam ? cameraName(cam) : shown)} {status} onrecenter={s.recenter}>
		{#snippet top()}
			<a class="all" href={listHref(page.url.searchParams)} data-testid="camera-all">{m['live_cameras.all']()}</a>
		{/snippet}
		{#snippet actions()}
			<button type="button" class="secondary" onclick={s.leave} data-testid="camera-leave">{m['live_cameras.leave']()}</button>
		{/snippet}
		{#if s.status === 'connected'}
			<CameraAsk hold={s.holding.hold} other={s.holding.other ?? (cam?.holder ? (cam.nick ?? cam.holder) : undefined)} refused={s.holding.refused} ontake={() => s.claim(shown)} onleave={s.leave} />
		{/if}
	</HandheldStage>
{:else}
	<HandheldStage words={w} hand={{ down() {}, move() {}, up() {}, startXR() {}, startOrientation() {} }} mode="none" tracking xr={false} live={false} big={id} {status} />
{/if}

<style>
	.all {
		margin-left: auto;
		color: var(--paper);
		pointer-events: auto;
	}
</style>
