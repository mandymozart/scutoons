<script lang="ts">
	// #478, owner 2026-11-11: "We can have an overlay full-screen icon in svelte top bottom corner that will go to
	// full-screen and hence hide the top menu and logo. So its cleaner exit with back". The browser deck's "stage only":
	// the page's own state on a history entry of its own (shallow routing), so the browser's back, the phone's back
	// gesture and Esc leave it – also where a page has no Fullscreen API (iPhone Safari); the Fullscreen API on top where
	// the browser has it. Page chrome only: the deck page covers the reader's menu and logo while `page.state` says so.
	// Corner (proposal): bottom left – top left is the logo, top right the menu and the draft mark, bottom right the
	// pairing card; on the deck with controls it sits above the left stick (`raised`).
	import { pushState } from '$app/navigation';
	import { page } from '$app/state';
	import { Icon } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';

	let { raised = false }: { raised?: boolean } = $props();

	const on = $derived(!!page.state.deckStageOnly);

	function enter() {
		pushState('', { ...page.state, deckStageOnly: true });
		// not on every browser (iPhone Safari), or refused: the page's own state is enough
		document.documentElement.requestFullscreen?.().catch(() => {});
	}

	/** back: the entry goes, `on` with it */
	function leave() {
		if (on) history.back();
	}

	// the browser's own full screen ends (its Esc, its gesture): the stage-only entry goes too; the entry left by back:
	// the browser's full screen ends with it
	$effect(() => {
		const changed = () => {
			if (!document.fullscreenElement) leave();
		};
		document.addEventListener('fullscreenchange', changed);
		return () => document.removeEventListener('fullscreenchange', changed);
	});
	$effect(() => {
		if (!on && document.fullscreenElement) void document.exitFullscreen().catch(() => {});
	});

	/** Esc where the browser's full screen did not take it – unless the deck took the key (its Live page closes with Esc) */
	function key(e: KeyboardEvent) {
		if (e.key !== 'Escape' || !on || document.fullscreenElement) return;
		setTimeout(() => {
			if (!e.defaultPrevented) leave();
		});
	}
</script>

<svelte:window onkeydown={key} />

<div class="corner sc-tool" class:raised>
	<button
		type="button"
		class="sc-round"
		title={on ? m['live_deck.stage_leave']() : m['live_deck.stage_only']()}
		aria-label={on ? m['live_deck.stage_leave']() : m['live_deck.stage_only']()}
		aria-pressed={on}
		data-testid="deck-stage-only"
		onclick={on ? leave : enter}
	>
		<Icon name={on ? 'minimize-2' : 'maximize-2'} />
	</button>
</div>

<style>
	.corner {
		position: absolute;
		left: 0.5rem;
		bottom: calc(0.5rem + env(safe-area-inset-bottom));
		z-index: 2;
		background: none;
	}
	/* above the panel's bar and the left stick (deck-sticks.svelte: 7.5rem, its bottom over the bar) */
	.corner.raised {
		bottom: calc(var(--control-height) + 0.75rem + 7.5rem + 0.5rem + env(safe-area-inset-bottom));
	}
</style>
