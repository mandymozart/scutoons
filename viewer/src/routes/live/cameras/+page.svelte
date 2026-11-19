<script lang="ts">
	// /live/cameras?live=<code> (research/LIVE-CAMERAS.md §2): the cameras of a live session this phone may take, as the
	// relay shows them (its filter decides), with who holds each now; one opens the camera's own page.
	import { onDestroy } from 'svelte';
	import { page } from '$app/state';
	import { LIVE_QUIC, LIVE_URL } from '$lib/config.ts';
	import { CameraSession } from '$lib/live-camera.svelte.ts';
	import { cameraHref, cameraName } from '$lib/live-cameras.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { sessionOf } from '$lib/reader-session.ts';

	const relay = sessionOf(page.url.searchParams, LIVE_URL, LIVE_QUIC);
	const s = relay ? new CameraSession(relay) : null;
	onDestroy(() => s?.close());
	const status = $derived(!s ? m['live_cameras.no_session']() : s.status === 'joining' ? m['live_cameras.connecting']() : s.status === 'offline' ? m['live_cameras.offline']() : '');
</script>

<svelte:head><title>{m['live_cameras.title']()}</title><meta name="theme-color" content="#000000" /></svelte:head>

<main class="cameras sc-tool" data-testid="live-cameras">
	<h1>{m['live_cameras.title']()}</h1>
	{#if status}<p role="status">{status}</p>{/if}
	{#if s?.status === 'connected'}
		{#if s.cameras.length}
			<p class="hint">{m['live_cameras.list_hint']()}</p>
			<ul>
				{#each s.cameras as c (c.id)}
					<li>
						<a href={cameraHref(c.id, page.url.searchParams)} data-testid="camera-row">
							<span class="name">{cameraName(c)}</span>
							<span class="who">{!c.holder ? m['live_cameras.free']() : c.away ? m['live_cameras.away']({ name: c.nick ?? c.holder }) : m['live_cameras.held_by']({ name: c.nick ?? c.holder })}</span>
						</a>
					</li>
				{/each}
			</ul>
		{:else}
			<p class="hint">{m['live_cameras.none']()}</p>
		{/if}
	{/if}
</main>

<style>
	:global(body) {
		margin: 0;
		background: var(--ink);
	}
	/* over the reader's menu and logo (main-menu.svelte), as the deck page */
	.cameras {
		position: fixed;
		inset: 0;
		z-index: 1005;
		overflow-y: auto;
		box-sizing: border-box;
		padding: var(--space-1);
		background: var(--ink);
		color: var(--paper);
		font-family: var(--font-family-mono, monospace);
	}
	h1 {
		margin: 0 0 var(--space-1);
		font-size: var(--type-title-size, 1.5rem);
	}
	.hint {
		opacity: 0.8;
	}
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	li a {
		display: flex;
		justify-content: space-between;
		gap: var(--space-1);
		padding: var(--space-1) 0;
		border-bottom: var(--border-width, 1px) solid color-mix(in srgb, var(--paper) 30%, transparent);
		color: var(--paper);
		text-decoration: none;
	}
	.who {
		opacity: 0.8;
		text-align: right;
	}
</style>
