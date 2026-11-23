<script lang="ts">
	import { tick, untrack } from 'svelte';
	import { fade } from 'svelte/transition';
	import { motionMs } from '@scutoons/ui/editor';
	import { goto } from '$app/navigation';
	import { getCreatorSession } from '$lib/session.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { recent } from '$lib/recent.svelte.ts';
	import { firstRing, greetName, isWelcome, rowStep, stepRing, workspaceCards, type WorkspaceCard } from '$lib/shell/dashboard.ts';
	import { shell } from '$lib/shell/shell.svelte.ts';
	import { crafts } from '$lib/shell/crafts.svelte.ts';
	import { layouts } from '$lib/shell/layouts.svelte.ts';
	import DashboardBoxes from '$lib/components/shell/dashboard-boxes.svelte';
	import Launcher from '$lib/components/shell/launcher.svelte';
	import WorkspaceCardView from '$lib/components/shell/workspace-card.svelte';

	/**
	 * The dashboard – the app switcher (research/redesign/lap/crafts-v2.html views `dash`, `first`): over everything, the
	 * stage dimmed and blurred under it; the workspace row's ⌄ or Esc goes back to where you were. On top the launcher
	 * (inline), then Last opened · Needs review / Not published, then the workspaces – flat, the front one first, then most
	 * used – each showing what is open in it. Tab / Shift Tab and the arrows move the ring, Enter opens; typing finds.
	 * Nothing used yet: the welcome – Not published in her toons, her crafts' workspaces empty. Nothing explains itself.
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
		// the launcher over everything takes its own keys
		if (!box || shell.launcher) return;
		// a menu open inside: its own Escape and keys
		if (box.querySelector('[aria-expanded="true"]')) return;
		const active = document.activeElement;
		if (event.key === 'Escape') {
			event.preventDefault();
			event.stopPropagation();
			shell.dashboard = false;
			return;
		}
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
		// typing finds: the launcher field takes the key
		if (event.key.length === 1 && event.key !== ' ') box.querySelector<HTMLInputElement>('[data-testid="launcher-input"]')?.focus();
	}
</script>

<svelte:window onkeydowncapture={onKeydown} />

<div class="dashboard" bind:this={box} role="dialog" aria-modal="true" aria-label={m['shell.dashboard']()} data-testid="dashboard" out:fade={{ duration: motionMs('base') }}>
	<div class="body">
		{#if welcome}<h1 class="hello" data-testid="dash-welcome">{m['shell.dash_welcome']({ name: greetName(session.user?.email ?? '') })}</h1>{/if}
		<div class="find"><Launcher inline onpick={() => (shell.dashboard = false)} /></div>
		<DashboardBoxes {welcome} />
		<h2 class="label">{m['shell.workspaces']()}</h2>
		<div class="cards" bind:this={grid}>
			{#each cards as card, i (card.id)}
				<WorkspaceCardView {card} ring={i === ring} onopen={open} onclose={closeItem} onfocus={() => (ring = i)} />
			{/each}
		</div>
	</div>
</div>

<style>
	/* over the stage, the panels and the status bar, under the bar's row (the workspace row's ⌄ stays to go back) */
	.dashboard {
		position: fixed;
		z-index: 48;
		inset: 0;
		display: flex;
		flex-direction: column;
		box-sizing: border-box;
		padding-top: var(--shell-top);
		/* the stage's own ground, nearly opaque and well blurred: a bright 3D stage or a panel must not read through (production 2026-11-24) */
		background: color-mix(in srgb, var(--tool-ground) 88%, transparent);
		backdrop-filter: blur(1.6rem);
		color: var(--tool-text);
		font-family: var(--editor-sans);
		animation: dash-in var(--motion-slow) var(--motion-ease);
	}
	/* the cards rise into place as the dashboard opens */
	.body {
		animation: dash-rise var(--motion-slow) var(--motion-ease);
		display: flex;
		flex: 1;
		flex-direction: column;
		gap: var(--editor-gap);
		min-height: 0;
		padding: 0 var(--editor-gap) var(--editor-gap);
		overflow: auto;
	}
	.hello {
		margin: var(--editor-gap) 0 0;
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
		grid-template-columns: repeat(auto-fill, minmax(18rem, 1fr));
		gap: var(--editor-gap);
	}
	@keyframes dash-rise {
		from {
			transform: translateY(0.5rem);
		}
	}
	@keyframes dash-in {
		from {
			opacity: 0;
		}
	}
</style>
