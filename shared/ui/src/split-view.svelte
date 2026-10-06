<script lang="ts">
	import type { Snippet } from 'svelte';
	import CloseButton from './close-button.svelte';
	import DockFold from './dock-fold.svelte';
	import { panelWidth } from './panel-width.svelte.ts';
	import { shellDock, useDock } from './dock.svelte.ts';

	/**
	 * List + detail for fast triage (admin catalog, codes): the list is the page, the detail opens in the AppShell's
	 * right dock (agents/DOCKING.md – `useDock`): the list makes room, the dock's edge drags it wider (remembered once
	 * for every list detail, `sc-panel-width:detail`), it folds to a rail, it takes turns with the Info panel; on phones
	 * it covers the page. `open` = a detail is shown; `onclose` fires when the user closes it (✕, Escape, or another
	 * panel taking the dock – the app clears its selection / URL parameter).
	 */
	let {
		open = false,
		title,
		onclose,
		closeLabel = 'Close',
		foldLabel,
		maximizeLabel,
		restoreLabel,
		list,
		detail,
		actions
	}: {
		open?: boolean;
		title?: string;
		onclose: () => void;
		closeLabel?: string;
		/** the dock's fold button */
		foldLabel?: string;
		/** the dock's maximize button (maximized / restore) */
		maximizeLabel?: string;
		restoreLabel?: string;
		list: Snippet;
		detail: Snippet;
		/** extra header actions of the detail (e.g. "Open page") */
		actions?: Snippet;
	} = $props();

	const size = panelWidth('sc-panel-width:detail', 24, undefined, { foldable: true });
	useDock(() => (open ? { panel: pane, size, title: title ?? '', icon: 'sidebar', close: () => onclose() } : null));

	function keydown(event: KeyboardEvent) {
		// maximized: Escape restores first (AppShell)
		if (event.key === 'Escape' && open && !shellDock.maximized) {
			event.stopPropagation();
			onclose();
		}
	}
</script>

<div class="sc-split">{@render list()}</div>

{#snippet pane()}
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<section class="detail" aria-label={title} onkeydown={keydown} data-testid="split-detail">
		<header>
			{#if title}<h2 {title}>{title}</h2>{/if}
			<div class="actions">
				{#if actions}{@render actions()}{/if}
				<DockFold label={foldLabel} {maximizeLabel} {restoreLabel} />
				<CloseButton label={closeLabel} onclick={onclose} />
			</div>
		</header>
		<div class="body">{@render detail()}</div>
	</section>
{/snippet}

<style>
	.sc-split {
		min-width: 0;
	}
	/* fills the dock (AppShell); the body scrolls */
	.detail {
		display: flex;
		flex-direction: column;
		min-height: 0;
		height: 100%;
		background: var(--paper);
	}
	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-05);
		flex: none;
		padding: var(--space-05) var(--space-1);
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	/* one line: the actions keep their room, a long title ends in … (its full text is the dock's name) */
	h2 {
		flex: 1;
		min-width: 0;
		margin: 0;
		font-size: var(--type-label-size);
		line-height: var(--type-label-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.actions {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		margin-left: auto;
	}
	.body {
		flex: 1;
		min-height: 0;
		padding: var(--space-1);
		overflow: auto;
		overscroll-behavior: contain;
	}
</style>
