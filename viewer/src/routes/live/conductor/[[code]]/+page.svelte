<script lang="ts">
	// /live/conductor (#231, research/LIVE.md B.2 / B.7): the conductor's vote – open a vote, watch the tally, Hold / Close
	// now / Override, the result. The relay as /live/vote (hosted by default, `?relay=` for a local one, the code from the path
	// or `?code=`); the conductor says its session token, `?token=` (B.6: the API's `POST /api/live/sessions` hands it out;
	// a relay without a secret takes any non-empty one). Unlisted; the relay refuses a wrong token. The web conductor
	// (B.7) will hold more than votes – this is its vote panel on its own page.
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { LIVE_QUIC, LIVE_URL } from '$lib/config.ts';
	import { WASM_BASE, relayOf } from '$lib/live-deck.ts';
	import { loadRelay } from '@scutoons/live-deck';
	import { RelayVote, type LinkStatus } from '$lib/live-vote-link.ts';
	import ConductorVote from '$lib/components/live-vote/conductor-vote.svelte';
	import OpenVoteForm from '$lib/components/live-vote/open-vote-form.svelte';
	import type { VoteOpen } from '@scutoons/live';

	let source = $state<RelayVote | null>(null);
	let status = $state<LinkStatus>({ state: 'joining' });
	let isOpen = $state(false);
	const key = $derived(page.url.searchParams.get('key')?.trim() ?? '');
	let seq = $state(1);

	onMount(() => {
		if (!key) return;
		const link = new RelayVote({ relay: relayOf(page.url.searchParams, LIVE_URL, page.params.code, LIVE_QUIC), role: 'conductor', token: key, core: loadRelay(WASM_BASE), onStatus: (s) => (status = s) });
		source = link;
		const off = link.on('conductor', (e) => {
			if (e.kind === 'vote.open') isOpen = true;
			else if (e.kind === 'vote.result') isOpen = false;
			else if (e.kind === 'error' && e.p.code === 'vote') problem = e.p.message;
		});
		return () => (off(), link.dispose());
	});
	let problem = $state('');
	function open(p: VoteOpen) {
		problem = '';
		source?.open(p);
		seq++;
	}
</script>

<svelte:head><title>Conductor – vote</title></svelte:head>

<div class="lab">
	<h1>Vote</h1>
	{#if !key}
		<p class="note">The conductor needs its session token: open this page with <code>?key=…</code>.</p>
	{:else}
		<p class="caption" role="status">
			{status.state === 'connected' ? `connected as ${status.client}` : status.state === 'refused' ? `refused: ${status.why}` : status.state === 'offline' ? 'offline – joining again …' : 'joining …'}
		</p>
		{#if problem}<p class="note" role="alert">{problem}</p>{/if}
		{#if source}
			<ConductorVote {source} />
			{#if !isOpen}<OpenVoteForm onopen={open} next={`v${seq}`} />{/if}
		{/if}
	{/if}
</div>

<style>
	.lab {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		box-sizing: border-box;
		max-width: 60rem;
		margin: 0 auto;
		padding: 5rem var(--space-1) var(--space-3);
		font-family: var(--font-mono);
	}
	h1 {
		margin: 0;
		font-size: var(--type-title-size);
	}
	.caption,
	.note {
		margin: 0;
		font-size: var(--type-caption-size);
		color: var(--text-muted);
	}
	.note {
		padding-left: var(--space-05);
		border-left: 0.5rem solid var(--badge-warning);
		color: var(--ink);
	}
</style>
