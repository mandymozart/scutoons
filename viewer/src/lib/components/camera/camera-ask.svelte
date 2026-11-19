<script lang="ts">
	// The camera page's word on the hold (research/LIVE-CAMERAS.md §5): ask first, naming a take-over; then Leave, or the
	// warning and Take it back. Every state is the core's (deck-space live_cameras `Holding`); this only shows it.
	import { m } from '$lib/paraglide/messages.js';

	interface Props {
		hold: 'none' | 'asked' | 'held' | 'taken_over';
		/** another phone holds the camera now (its name) */
		other?: string;
		refused?: string;
		ontake: () => void;
		onleave: () => void;
	}
	const { hold, other, refused, ontake, onleave }: Props = $props();
</script>

{#if hold === 'none'}
	<div class="ask" role="dialog" aria-labelledby="camera-question" data-testid="camera-ask">
		<p id="camera-question">{m['live_cameras.question']()}</p>
		{#if other}<p class="warn" data-testid="camera-take-over">{m['live_cameras.take_over']({ name: other })}</p>{/if}
		{#if refused}<p class="warn" data-testid="camera-refused">{m['live_cameras.refused']()}</p>{/if}
		<div class="buttons">
			<button type="button" onclick={ontake} data-testid="camera-yes">{m['live_cameras.yes']()}</button>
		</div>
	</div>
{:else if hold === 'asked'}
	<p class="ask" role="status">{m['live_cameras.asked']()}</p>
{:else if hold === 'taken_over'}
	<div class="ask" role="alert" data-testid="camera-taken-over">
		<p class="warn">{m['live_cameras.taken_over']({ name: other ?? '' })}</p>
		<div class="buttons">
			<button type="button" class="secondary" onclick={onleave} data-testid="camera-leave">{m['live_cameras.leave']()}</button>
			<button type="button" onclick={ontake} data-testid="camera-take-back">{m['live_cameras.take_back']()}</button>
		</div>
	</div>
{/if}

<style>
	.ask {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		width: 100%;
		max-width: 22rem;
		box-sizing: border-box;
		margin: var(--space-1) 0 0;
		padding: var(--space-1);
		border: var(--border-width, 1px) solid var(--paper);
		pointer-events: auto;
	}
	.ask p {
		margin: 0;
	}
	.warn {
		color: var(--state-warning, var(--paper));
	}
	.buttons {
		display: flex;
		gap: var(--space-05);
		justify-content: center;
	}
	.buttons button {
		flex: 1 1 0;
		min-width: 0;
		margin: 0;
		padding-inline: var(--space-05);
		background: var(--paper);
		color: var(--ink);
		border-color: var(--paper);
	}
	.buttons button.secondary {
		background: var(--ink);
		color: var(--paper);
		border: var(--border-width, 1px) solid var(--paper);
	}
</style>
