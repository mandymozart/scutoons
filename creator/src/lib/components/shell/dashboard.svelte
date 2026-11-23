<script lang="ts">
	import { tick, untrack } from 'svelte';
	import { Overlay } from '@scutoons/ui/editor';
	import { goto } from '$app/navigation';
	import { getCreatorSession } from '$lib/session.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { recent } from '$lib/recent.svelte.ts';
	import { firstRing, greetName, isWelcome, rowStep, stepRing, workspaceCards, type WorkspaceCard } from '$lib/shell/dashboard.ts';
	import { shell } from '$lib/shell/shell.svelte.ts';
	import { crafts } from '$lib/shell/crafts.svelte.ts';
	import { layouts } from '$lib/shell/layouts.svelte.ts';
	import DashboardBoxes from '$lib/components/shell/dashboard-boxes.svelte';
	import WorkspaceCardView from '$lib/components/shell/workspace-card.svelte';

	/**
	 * The switcher (lap2 A3, RULES H15): on the overlay level – the bar, the stage and the panels blurred under it; no bar,
	 * no search field, no foot of its own. Esc, a click outside, Ctrl K or the workspace row's ⌄ go back to where you were.
	 * Last opened · Needs review / Not published, then the workspaces – flat, the front one first, then most used – each
	 * showing what is open in it; at most 1160 px wide, centred. Tab / Shift Tab and the arrows move the ring, Enter opens;
	 * a typed letter opens Search with it. Nothing used yet: the welcome – Not published in her toons, her crafts'
	 * workspaces empty.
	 */
	const session = getCreatorSession();
	const welcome = $derived(isWelcome(shell.use, shell.state.tabs, recent.list.length));
	const cards = $derived(workspaceCards(shell.use, shell.state.tabs, shell.state.front, welcome, crafts.starts, layouts.state.own, layouts.chosen));
	let ring = $state(0);
	let box = $state<HTMLElement>();
	let grid = $state<HTMLElement>();

	const cardEls = () => [...(grid?.querySelectorAll<HTMLElement>('[data-testid="dash-card-open"]') ?? [])];
	function focusRing(at: number) {
		ring = at;
		cardEls()[at]?.focus();
	}
	// opened: the ring on the card after the front one (app-switcher style)
	$effect(() => {
		void tick().then(() => untrack(() => focusRing(firstRing(cards))));
	});
	$effect(() => {
		if (ring >= cards.length) ring = Math.max(0, cards.length - 1);
	});

	function open(card: WorkspaceCard) {
		shell.dashboard = false;
		layouts.show(card.kind, card.id);
		if (!card.href) shell.open('library');
	}
	function closeItem(address: string) {
		const next = shell.close(address);
		if (next) void goto(next);
		void tick().then(() => focusRing(Math.min(ring, cards.length - 1)));
	}

	const columns = () => (grid ? getComputedStyle(grid).gridTemplateColumns.split(' ').filter(Boolean).length : 1);
	const typing = (el: Element | null) => el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement || (el instanceof HTMLElement && el.isContentEditable);

	function onKeydown(event: KeyboardEvent) {
		if (!box || shell.launcher) return;
		// a menu open inside, or a dialog over it: their own keys
		if (box.querySelector('[aria-expanded="true"]') || document.querySelector('dialog[open]')) return;
		const active = document.activeElement;
		if (event.ctrlKey || event.metaKey || event.altKey) return;
		const count = cards.length;
		if (event.key === 'Tab') {
			event.preventDefault();
			const onCard = cardEls().includes(active as HTMLElement);
			focusRing(onCard ? stepRing(ring, event.shiftKey ? -1 : 1, count) : ring);
			return;
		}
		if (typing(active)) return;
		const step = { ArrowRight: 1, ArrowLeft: -1 }[event.key];
		if (step) {
			event.preventDefault();
			focusRing(stepRing(ring, step, count));
			return;
		}
		if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
			event.preventDefault();
			focusRing(rowStep(ring, event.key === 'ArrowDown' ? 1 : -1, count, columns()));
			return;
		}
		// typing finds: Search takes the switcher's place, the letter in its field
		if (event.key.length === 1 && event.key !== ' ') {
			event.preventDefault();
			shell.toggleLauncher(true, event.key);
		}
	}
</script>

<svelte:window onkeydowncapture={onKeydown} />

<Overlay place="top" label={m['shell.dashboard']()} onclose={() => shell.toggleSwitcher(false)} data-testid="dashboard">
	<div class="body" bind:this={box}>
		{#if welcome}<h1 class="hello" data-testid="dash-welcome">{m['shell.dash_welcome']({ name: greetName(session.user?.email ?? '') })}</h1>{/if}
		<DashboardBoxes {welcome} />
		<h2 class="label">{m['shell.workspaces']()}</h2>
		<div class="cards" bind:this={grid}>
			{#each cards as card, i (card.id)}
				<WorkspaceCardView {card} ring={i === ring} onopen={open} onclose={closeItem} onfocus={() => (ring = i)} />
			{/each}
		</div>
	</div>
</Overlay>

<style>
	/* the column: at most 1160 px (82.85rem at the kit's 14 px), centred; the cards rise into place as it opens */
	.body {
		display: flex;
		flex-direction: column;
		gap: var(--editor-gap);
		box-sizing: border-box;
		width: min(82.85rem, 100%);
		margin: 0 auto;
		padding: calc(var(--shell-top, 4rem) + var(--editor-gap)) var(--editor-gap) var(--editor-gap);
		animation: dash-rise var(--motion-slow) var(--motion-ease);
	}
	.hello {
		margin: 0;
		padding: 0 calc(var(--editor-pad) / 2);
		font-size: 1.4rem;
		font-weight: inherit;
		text-shadow: var(--tool-halo);
	}
	.label {
		margin: 0;
		padding: 0 calc(var(--editor-pad) / 2);
		color: var(--tool-muted);
		font-size: var(--editor-label);
		font-weight: inherit;
		letter-spacing: var(--editor-label-spacing);
		text-transform: uppercase;
		text-shadow: var(--tool-halo);
	}
	.cards {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(12rem, 1fr));
		gap: var(--editor-gap);
	}
	@keyframes dash-rise {
		from {
			transform: translateY(0.5rem);
		}
	}
</style>
