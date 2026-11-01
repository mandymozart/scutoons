<script lang="ts">
	// /live/vote (#231, research/LIVE.md B.9): the audience's phone votes. The relay: the hosted one, wss://live.scutoons.com
	// /live/<code> (VITE_LIVE_URL), the code from the path `/live/vote/<code>` or `?code=` (default CATH); `?relay=ws://…/live/CODE`
	// joins a local relay (lib/live-deck.ts `relayOf`, the same as the browser deck). `?group=` = the group (the QR carries it);
	// `?lang=` overrides the device's language. Unlisted, open to everyone (B.2: the audience joins with the code).
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { LIVE_URL } from '$lib/config.ts';
	import { relayOf } from '$lib/live-deck.ts';
	import { deviceLocale, groupLook } from '$lib/live-vote.ts';
	import { RelayVote, type LinkStatus } from '$lib/live-vote-link.ts';
	import AudienceVote from '$lib/components/live-vote/audience-vote.svelte';

	const query = $derived(page.url.searchParams);
	const group = $derived(query.get('group')?.trim() || 'audience');
	let source = $state<RelayVote | null>(null);
	let status = $state<LinkStatus>({ state: 'joining' });
	const locale = $derived(query.get('lang') ?? deviceLocale(typeof navigator === 'undefined' ? undefined : navigator.languages));

	onMount(() => {
		const link = new RelayVote({ relay: relayOf(page.url.searchParams, LIVE_URL, page.params.code), role: 'audience', group, onStatus: (s) => (status = s) });
		source = link;
		return () => link.dispose();
	});
</script>

<svelte:head><title>Vote</title></svelte:head>

<div class="screen">
	{#if source}
		{#key group}
			<AudienceVote {source} group={{ id: group, name: group, ...groupLook(group) }} {locale} client="me" />
		{/key}
	{/if}
	{#if status.state !== 'connected'}
		<p class="status" role="status">{status.state === 'refused' ? status.why : status.state === 'offline' ? 'offline – joining again …' : 'joining …'}</p>
	{/if}
</div>

<style>
	.screen {
		display: flex;
		flex-direction: column;
		min-height: 100dvh;
		position: relative;
	}
	/* the reader's fixed logo and menu sit over the top 4rem */
	.screen :global(.phone) {
		padding-top: 4.5rem;
	}
	.status {
		position: absolute;
		inset: auto var(--space-1) var(--space-1);
		margin: 0;
		padding: var(--space-05);
		background: var(--paper);
		color: var(--ink);
		border: var(--border-width-regular) solid var(--ink);
		font-family: var(--font-mono);
		font-size: var(--type-caption-size);
	}
</style>
