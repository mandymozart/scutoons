<script lang="ts">
	// Dev-only draft of the cave – the avatar's home (research/CAVE.md D1–D7, FEATURES 10.98): the map with the
	// avatar walking home (or the VR headset), the room with furniture spots, a friend's visit. Nothing is saved.
	import CaveMap from './cave-map.svelte';
	import CaveRoom from './cave-room.svelte';

	const isDev = import.meta.env.DEV;
	let vr = $state(false);
	let inside = $state<null | 'own' | 'visit'>(null);
	let how = $state('');
</script>

{#if isDev}
	<div class="view lab">
		<h1>Cave (draft)</h1>
		{#if inside}
			<p class="muted" data-testid="cave-where">
				{inside === 'visit' ? "Visiting Mia's cave (from her visit link)" : how === 'vr' ? 'Home – with the VR headset (your avatar stays on the map)' : 'Home'}
			</p>
			<CaveRoom visitor={inside === 'visit'} onleave={() => (inside = null)} />
		{:else}
			<div class="toggles">
				<label><input type="checkbox" bind:checked={vr} data-testid="cave-has-vr" /> I own a VR headset</label>
				<button onclick={() => (inside = 'visit')} data-testid="cave-visit">Open a friend's visit link</button>
			</div>
			<CaveMap {vr} onenter={(h) => ((how = h), (inside = 'own'))} />
		{/if}
	</div>
{/if}

<style>
	.lab {
		display: grid;
		gap: 0.75rem;
		padding: 4.5rem 1rem 1rem; /* below the fixed logo and menu */
		max-width: 80rem;
		margin-inline: auto;
	}
	h1 {
		margin: 0;
	}
	.muted {
		margin: 0;
		color: var(--muted);
	}
	.toggles {
		display: flex;
		flex-wrap: wrap;
		gap: 1rem;
		align-items: center;
	}
	.toggles label {
		display: inline-flex;
		gap: 0.5rem;
		align-items: center;
		white-space: nowrap;
	}
	.toggles button {
		min-height: var(--control-height);
		padding: 0 0.75rem;
		border: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
		color: var(--ink);
		font: inherit;
		cursor: pointer;
	}
</style>
