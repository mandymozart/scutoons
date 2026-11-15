<script lang="ts">
	// /live/ballot (#549, research/SHOW.md §15 point 3b): the phone's voting screen, drawn and decided by the deck – the
	// browser deck's audience start on its light start (space-web `VoteView`: the ballot on the deck's 2D, no scene, no
	// sound). The address is the audience page's: `/live/ballot/<code>?group=` (the code from the path or `?code=`, default
	// CATH; `?relay=ws://…/live/CODE` joins a local relay – lib/live-deck.ts `relayOf`; `?lang=` overrides the device's
	// language; `?gl=1` WebGL2 only). Until #557 the printed `/live/vote/<code>?group=` still opens the Svelte page; #557
	// moves this file there (RULES I1: the printed address then starts the deck).
	// Here only what the deck cannot reach on a phone: the element, the address, the reader's sign-in for the proof's fetch,
	// and the wake lock – held while the core says a vote can come.
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { getSessionToken } from '@scutoons/api';
	import { spaceModule } from '@scutoons/live-deck';
	import { VoteStage } from '@scutoons/live-deck/vote-stage';
	import { API_URL, LIVE_QUIC, LIVE_URL } from '$lib/config.ts';
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { WASM_BASE, nickOf, preferOf, relayOf } from '$lib/live-deck.ts';
	import { wakeLock } from '$lib/wake-lock.svelte.ts';

	const session = getSessionContext();
	let stage = $state<HTMLDivElement>();

	onMount(() => {
		let alive = true;
		let vote: VoteStage | null = null;
		let release: (() => void) | null = null;
		const q = page.url.searchParams;
		// the wasm loads while the session answers
		void spaceModule(WASM_BASE).catch(() => {});
		void (async () => {
			// it joins under the signed-in reader's name: the session's first answer, at most a moment
			for (let i = 0; i < 40 && !session.ready; i++) await new Promise((r) => setTimeout(r, 50));
			if (!alive || !stage) return;
			try {
				const s = await VoteStage.start({
					host: stage,
					base: WASM_BASE,
					api: API_URL,
					relay: relayOf(q, LIVE_URL, page.params.code, LIVE_QUIC),
					group: q.get('group')?.trim() ?? '',
					locale: q.get('lang') ?? navigator.language,
					reader: nickOf(session.me, ''),
					prefer: preferOf(q),
					user: () => (session.isLoggedIn ? { bearer: getSessionToken() } : null),
					onAwake: (awake) => {
						release?.();
						release = awake ? wakeLock.hold('live-vote') : null;
					}
				});
				if (alive) vote = s;
				else s.stop();
			} catch (e) {
				console.warn('live vote: the voting screen did not start', e);
			}
		})();
		return () => {
			alive = false;
			vote?.stop();
			release?.();
		};
	});
</script>

<svelte:head><title>Vote</title></svelte:head>

<div class="screen" bind:this={stage} data-testid="live-ballot"></div>

<style>
	.screen {
		position: fixed;
		inset: 0;
		background: var(--ink);
		overflow: hidden;
	}
</style>
