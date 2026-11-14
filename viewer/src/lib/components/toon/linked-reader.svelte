<script lang="ts">
	// #528 (agents/research/SCENE-EVENTS.md §6; owner 2026-11-13 [scenes]: "Yes, ask first." · "Yes show a small linked
	// sign" · the take-over warning: "The newcomer"): a reader page with `?live=<code>` joins its session without claiming
	// and asks once whether this reading drives the stage; then the small linked sign. Every state is the core's (deck-space
	// linked.rs `Driving`, relay.rs `Out::Driving`): the page shows it and calls `drive()` (yes, take back) or leaves (no,
	// leave). Leaving the toon's pages leaves the session – no link the person can't see.
	import { onDestroy, onMount } from 'svelte';
	import { page } from '$app/state';
	import { LIVE_QUIC, LIVE_URL } from '$lib/config.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { joinSession, leaveSession, sessionOf, type Driving, type ReaderLink } from '$lib/reader-session.ts';

	let link: ReaderLink | null = null;
	let driving = $state<Driving | null>(null);

	onMount(() => {
		const relay = sessionOf(page.url.searchParams, LIVE_URL, LIVE_QUIC);
		if (relay) link = joinSession(relay, { onOther: (o) => void ('driving' in o && (driving = o.driving)) });
	});

	function leave() {
		leaveSession();
		link = null;
		driving = null;
	}
	onDestroy(leave);
</script>

{#if driving?.claim === 'none'}
	<div class="linked" role="dialog" aria-labelledby="linked-reader-question" data-testid="linked-reader-ask">
		<div class="words">
			<p id="linked-reader-question">{m['linked_reader.question']()}</p>
			{#if driving.other}<p class="warn" data-testid="linked-reader-take-over">{m['linked_reader.take_over']()}</p>{/if}
		</div>
		<button class="btn-secondary" onclick={leave} data-testid="linked-reader-no">{m['linked_reader.no']()}</button>
		<button onclick={() => link?.drive()} data-testid="linked-reader-yes">{m['linked_reader.yes']()}</button>
	</div>
{:else if driving}
	<div class="linked sign" role="status" data-testid="linked-reader-sign" data-claim={driving.claim}>
		<span class="dot" class:off={driving.claim !== 'drives'} aria-hidden="true"></span>
		<span class="words">
			{driving.claim === 'drives' ? m['linked_reader.drives']() : driving.claim === 'taken_over' ? m['linked_reader.taken_over']() : m['linked_reader.asked']()}
		</span>
		{#if driving.claim === 'taken_over'}
			<button onclick={() => link?.drive()} data-testid="linked-reader-take-back">{m['linked_reader.take_back']()}</button>
		{/if}
		<button class="btn-secondary" onclick={leave} data-testid="linked-reader-leave">{m['linked_reader.leave']()}</button>
	</div>
{/if}

<style>
	/* the banner look of update-prompt.svelte; the sign is the same, smaller */
	.linked {
		position: fixed;
		z-index: 2030;
		left: 0.5rem;
		right: 0.5rem;
		bottom: 0.5rem;
		max-width: 31.25rem;
		margin: 0 auto;
		box-sizing: border-box;
		background: var(--paper);
		color: var(--ink);
		border: var(--border-width-regular, 2px) solid var(--ink);
		padding: 0.75rem;
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 0.5rem;
	}
	.words {
		flex: 1 1 12rem;
	}
	.words p {
		margin: 0;
	}
	.warn {
		margin-top: 0.25rem !important;
		font-size: 0.875rem;
	}
	.linked button {
		padding: 0 1rem;
	}
	.sign {
		left: auto;
		max-width: 20rem;
		padding: 0.25rem 0.25rem 0.25rem 0.75rem;
		font-size: 0.875rem;
	}
	.sign .words {
		flex: 1 1 auto;
	}
	.dot {
		width: 0.5rem;
		height: 0.5rem;
		border-radius: 50%;
		background: currentColor;
	}
	.dot.off {
		background: transparent;
		border: var(--border-width-regular, 2px) solid currentColor;
	}
</style>
