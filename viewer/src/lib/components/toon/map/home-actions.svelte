<script lang="ts">
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The place card's walking and home actions (research/CAVE.md D1, D2): "Walk here" to an open place the avatar can
	 * reach; at the avatar's home "Enter" (being there is free), else "Walk home" and – with a VR headset – "Enter with
	 * the VR headset". Rides (cucumbers, the networks) come in step 2.
	 */
	let {
		place,
		at,
		home,
		reachable,
		walking,
		vr,
		onwalk,
		onenter
	}: {
		/** the card's place */
		place: string;
		/** where the avatar stands */
		at: string | undefined;
		/** the avatar's home place */
		home: string | undefined;
		/** the avatar can walk to the card's place */
		reachable: boolean;
		walking: boolean;
		/** the reader owns a VR headset */
		vr: boolean;
		onwalk: () => void;
		onenter: (how: 'there' | 'vr') => void;
	} = $props();

	const isHome = $derived(!!home && place === home);
	const there = $derived(at === place);
</script>

{#if isHome || reachable || there}
	<div class="actions" data-testid="map-home-actions">
		{#if isHome}<p class="muted">{m['avatar_home.your_home']()}</p>{/if}
		{#if there && !isHome}<p class="muted">{m['avatar_home.avatar_here']()}</p>{/if}
		{#if isHome && there}
			<button class="sc-control primary" onclick={() => onenter('there')} data-testid="map-home-enter">{m['avatar_home.enter']()}</button>
		{:else if !there}
			{#if isHome && vr}<button class="sc-control" onclick={() => onenter('vr')} data-testid="map-home-vr">{m['avatar_home.enter_vr']()}</button>{/if}
			{#if reachable}
				<button class="sc-control" class:primary={!isHome || !vr} disabled={walking} onclick={onwalk} data-testid="map-walk">
					{walking ? m['avatar_home.walking']() : isHome ? m['avatar_home.walk_home']() : m['avatar_home.walk_here']()}
				</button>
			{/if}
		{/if}
	</div>
{/if}

<style>
	.actions {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.5rem;
	}
	.muted {
		flex-basis: 100%;
		margin: 0;
		color: var(--muted);
	}
	button {
		min-height: var(--control-height);
		padding: 0 0.75rem;
		border: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
		color: var(--ink);
		font: inherit;
		cursor: pointer;
	}
	button.primary {
		background: var(--ink);
		color: var(--paper);
	}
	button:disabled {
		opacity: 0.5;
		cursor: default;
	}
</style>
