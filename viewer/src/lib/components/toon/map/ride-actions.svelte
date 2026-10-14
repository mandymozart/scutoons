<script lang="ts">
	import { RIDE_PRICES, type MapNetwork } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The place card's rides (research/CAVE.md D8): one button per network that runs from the stop where the avatar
	 * stands to this place – "Ride the subway here · 2 cucumbers". The fare is paid when the ride starts.
	 */
	let {
		networks,
		busy,
		note,
		onride
	}: {
		/** the networks that reach this place from where the avatar stands */
		networks: MapNetwork[];
		busy: boolean;
		/** why the last ride didn't start ("You need 2 cucumbers") */
		note: string;
		onride: (network: MapNetwork) => void;
	} = $props();

	const label = (n: MapNetwork) =>
		({ subway: m['avatar_home.ride_subway'], tram: m['avatar_home.ride_tram'], airlift: m['avatar_home.ride_airlift'] })[n]({ price: RIDE_PRICES[n] });
</script>

{#if networks.length}
	<div class="rides" data-testid="map-rides">
		{#each networks as n (n)}
			<button class="sc-control" disabled={busy} onclick={() => onride(n)} data-testid="map-ride-{n}">{label(n)}</button>
		{/each}
		{#if note}<p class="note" role="status">{note}</p>{/if}
	</div>
{/if}

<style>
	.rides {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
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
	button:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.note {
		flex-basis: 100%;
		margin: 0;
	}
</style>
