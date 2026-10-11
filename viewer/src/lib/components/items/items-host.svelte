<script lang="ts">
	import { CloseIcon } from '@scutoons/ui';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { localizeHref } from '$lib/paraglide/runtime.js';
	import { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import { getItemsUiContext } from '$lib/contexts/items-ui.svelte.ts';
	import { getMembershipContext } from '$lib/contexts/membership.svelte.ts';
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import CameraOverlay from '$lib/components/camera/camera-overlay.svelte';
	import ClaimSheet from './claim-sheet.svelte';
	import ItemIcon from './item-icon.svelte';
	import { loadMinigame } from './minigames/registry.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import mapIcon from '$lib/icons/map.svg';
	import { replayPlaceReveal } from '$lib/components/toon/map/map-reveal.ts';

	/** Global item overlays (mounted once in the root layout). */
	const inventory = getInventoryContext();
	const ui = getItemsUiContext();
	const membership = getMembershipContext();
	const session = getSessionContext();

	// "X schenkt dir Y" / a code moved here after a claim (GET /me/ `received`, ITEMS.md "Ownership, claims, gifting")
	$effect(() => {
		const received = session.me?.received;
		if (received?.length) inventory.announceReceived(received);
	});

	// the album's tracks in order: a new request starts at the first, the end of a track plays the next
	let trackIndex = $state(0);
	$effect(() => {
		void ui.audio;
		trackIndex = 0;
	});
	function nextTrack() {
		if (ui.audio && trackIndex < ui.audio.tracks.length - 1) trackIndex += 1;
	}

	function openInventory() {
		ui.closeClaim();
		if (page.params.toonId) {
			ui.requestInventory();
		} else {
			goto(localizeHref('/catpope') + '?inventory=open');
		}
	}

	function noticeText(n: { item_id: string; qty: number; kind: string; email?: string | null; reason?: string; place?: { name: string } }) {
		if (n.kind === 'place') return m['map.place_unlocked']({ name: n.place?.name ?? '' });
		const name = inventory.nameOf({ item_id: n.item_id, variant: null });
		if (n.kind === 'refused') return refusedText(n.reason, name);
		const many = n.qty > 1 ? `${n.qty} × ${name}` : name;
		if (n.kind === 'gifted') return m['items.gift_done']({ email: n.email ?? '', name: many });
		if (n.kind === 'gift_received') return m['items.gift_received']({ from: n.email ?? '', name: many });
		if (n.kind === 'now_yours') return m['items.now_yours']({ name });
		if (n.kind === 'craft') return m['items.crafted']({ name });
		if (n.kind === 'claim') return m['items.claimed']({ name });
		if (n.kind === 'returned') return m['items.returned']({ name });
		if (n.kind === 'taken') return m['items.taken']({ qty: n.qty, name });
		if (n.kind === 'used') return (inventory.item(n.item_id)?.category === 'food' ? m['items.eaten'] : m['items.used'])({ name });
		return m['items.granted']({ qty: n.qty, name });
	}

	/** a placed item that stayed on the slice: why (the API's error code where it said one) */
	function refusedText(reason: string | undefined, name: string) {
		if (reason === 'max_reached') return m['items.pickup_max_reached']();
		// the world limit (max_world, BACKLOG 10.39): owned by as many accounts as the catalog allows
		if (reason === 'taken') return m['items.pickup_taken']();
		if (reason === 'not_grantable' || reason === 'unknown_item' || reason?.startsWith('respawn_') || reason === 'invalid_respawn') {
			return m['items.pickup_not_grantable']({ name });
		}
		if (reason === 'already_taken') return m['items.pickup_already_taken']({ name });
		if (reason === 'rate_limited' || reason === 'cooldown' || reason === 'daily_limit') return m['items.pickup_rate_limited']();
		return m['items.pickup_failed']({ name });
	}
</script>

{#if ui.camera}
	<CameraOverlay
		request={ui.camera}
		onclose={ui.closeCamera}
		oncode={(code) => {
			ui.closeCamera();
			ui.openClaim(code);
		}}
	/>
{/if}

{#if ui.claimCode}
	{#key ui.claimCode}
		<ClaimSheet code={ui.claimCode} auto={ui.claimAuto} dispute={ui.claimDispute} onclose={ui.closeClaim} onopeninventory={openInventory} />
	{/key}
{/if}

{#if ui.minigame}
	<div class="overlay" role="dialog" aria-modal="true">
		<div class="panel">
			{#await loadMinigame(ui.minigame.id) then mod}
				<mod.default entry={ui.minigame.entry} onclose={ui.closeMinigame} />
			{/await}
		</div>
	</div>
{/if}

{#if ui.audio}
	{@const track = ui.audio.tracks[Math.min(trackIndex, ui.audio.tracks.length - 1)]}
	<div class="audio-player" data-testid="audio-player">
		{#if ui.audio.entry}
			<div class="thumb">
				<ItemIcon srcs={[inventory.iconOf(ui.audio.entry)]} name={inventory.nameOf(ui.audio.entry)} />
			</div>
		{/if}
		<div class="track">
			{#if track.title || ui.audio.title}
				<span class="track-title" data-testid="audio-title">{m['items.now_playing']({ title: [ui.audio.title, track.title].filter(Boolean).join(' · ') })}</span>
			{/if}
			<!-- svelte-ignore a11y_media_has_caption -->
			<audio src={/^https?:/.test(track.src) ? track.src : asset(track.src)} controls autoplay onended={nextTrack}></audio>
		</div>
		<button class="close" onclick={ui.closeAudio} aria-label={m['items.close']()}><CloseIcon size="1rem" /></button>
	</div>
{/if}

{#if membership.invalidatedCode}
	<div class="banner" role="alert">
		<p>{m['claim.no_longer_valid']()} ({membership.invalidatedCode})</p>
		<button class="btn-secondary" onclick={membership.dismissInvalidated}>{m['items.close']()}</button>
	</div>
{/if}

<div class="notices" aria-live="polite">
	{#each inventory.notices as notice (notice.id)}
		{#if notice.kind === 'place' && notice.place?.id}
			<!-- a place's toast: a tap opens the map and plays its reveal (owner 2026-10-11) -->
			<button type="button" class="notice tap" onclick={() => replayPlaceReveal(notice.place!.id!)} data-testid="notice-place">
				<div class="thumb"><img class="place-thumb" src={notice.place.image ? asset(notice.place.image) : mapIcon} alt="" /></div>
				<span>{noticeText(notice)}</span>
			</button>
		{:else}
		<div class="notice">
			<div class="thumb">
				{#if notice.kind === 'place'}
					<img class="place-thumb" src={notice.place?.image ? asset(notice.place.image) : mapIcon} alt="" />
				{:else}
					<ItemIcon srcs={[inventory.item(notice.item_id)?.icon ?? '']} name={inventory.nameOf({ item_id: notice.item_id, variant: null })} />
				{/if}
			</div>
			<span>{noticeText(notice)}</span>
			{#if (notice.repeat ?? 1) > 1}<span class="repeat" data-testid="notice-repeat">× {notice.repeat}</span>{/if}
		</div>
		{/if}
	{/each}
</div>

<style>
	.overlay {
		position: fixed;
		inset: 0;
		z-index: 2050;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: flex-end;
		justify-content: center;
	}
	.panel {
		width: 100%;
		max-width: 500px;
		box-sizing: border-box;
		background: white;
		border: 2px solid black;
		border-bottom: none;
		padding: 1rem;
	}
	.audio-player {
		position: fixed;
		z-index: 2040;
		left: 0.5rem;
		right: 0.5rem;
		bottom: 0.5rem;
		max-width: 500px;
		margin: 0 auto;
		display: flex;
		align-items: center;
		gap: 0.5rem;
		background: white;
		border: 2px solid black;
		padding: 0.25rem 0.5rem;
	}
	.audio-player .track {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
	}
	.audio-player audio {
		width: 100%;
		min-width: 0;
	}
	.track-title {
		font-size: 0.75rem;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.thumb {
		width: 2.5rem;
		flex: none;
	}
	.place-thumb {
		display: block;
		width: 2.5rem;
		height: 2.5rem;
		object-fit: contain;
	}
	.close {
		box-sizing: border-box;
		width: var(--control-height);
		height: var(--control-height);
		padding: 0;
		border-radius: 50%;
		background: white;
		color: black;
		border: 2px solid black;
		display: inline-grid;
		place-items: center;
		flex: none;
	}
	.close:hover {
		background: var(--primary-color);
	}
	.banner {
		position: fixed;
		z-index: 2030;
		left: 0.5rem;
		right: 0.5rem;
		bottom: 0.5rem;
		max-width: 500px;
		margin: 0 auto;
		box-sizing: border-box;
		background: white;
		border: 2px solid black;
		padding: 0.75rem;
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}
	.banner p {
		margin: 0;
		flex: 1;
	}
	.banner button {
		padding: 0 1rem;
	}
	.notices {
		position: fixed;
		z-index: 2200;
		top: 3rem;
		left: 50%;
		transform: translateX(-50%);
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		pointer-events: none;
		width: max-content;
		max-width: calc(100vw - 2rem);
	}
	.repeat {
		flex: none;
		font-variant-numeric: tabular-nums;
		color: #6b6b6b;
	}
	.notice {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		background: white;
		border: 2px solid black;
		padding: 0.25rem 0.75rem 0.25rem 0.25rem;
		filter: drop-shadow(0 0 8px var(--primary-color));
		animation: notice-in 0.3s ease;
	}
	/* a place's toast is a button (a tap plays the reveal): the toast's look, not a button's – the global button colour
	   made its text white on white (owner 2026-10-11) – and tappable although the stack lets taps through */
	.notice.tap {
		font: inherit;
		color: var(--ink, #000);
		text-align: left;
		cursor: pointer;
		pointer-events: auto;
	}
	@keyframes notice-in {
		from {
			transform: translateY(-0.5rem);
			opacity: 0;
		}
		to {
			transform: none;
			opacity: 1;
		}
	}
</style>
