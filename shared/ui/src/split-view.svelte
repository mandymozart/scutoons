<script lang="ts">
	import type { Snippet } from 'svelte';
	import Sheet from './sheet.svelte';
	import CloseButton from './close-button.svelte';

	/**
	 * List + detail for fast triage. Wide screens (≥ `breakpoint` px): the detail is a pane next to the list (sticky,
	 * with a close button); narrower: a right drawer (modal Sheet). `open` = a detail is shown; `onclose` fires when the
	 * user closes it (the app clears its selection / URL parameter).
	 */
	let {
		open = false,
		title,
		onclose,
		closeLabel = 'Close',
		breakpoint = 1100,
		list,
		detail,
		actions
	}: {
		open?: boolean;
		title?: string;
		onclose: () => void;
		closeLabel?: string;
		breakpoint?: number;
		list: Snippet;
		detail: Snippet;
		/** extra header actions of the detail (e.g. "Open page") */
		actions?: Snippet;
	} = $props();

	let wide = $state(true);

	$effect(() => {
		const query = window.matchMedia(`(min-width: ${breakpoint}px)`);
		wide = query.matches;
		const change = (event: MediaQueryListEvent) => (wide = event.matches);
		query.addEventListener('change', change);
		return () => query.removeEventListener('change', change);
	});

	let sheetOpen = $state(false);
	$effect.pre(() => {
		sheetOpen = open && !wide;
	});

	function keydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && open && wide) {
			event.stopPropagation();
			onclose();
		}
	}
</script>

<div class="sc-split" class:with-detail={open && wide}>
	<div class="list">{@render list()}</div>
	{#if open && wide}
		<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
		<aside class="detail" aria-label={title} onkeydown={keydown}>
			<header>
				{#if title}<h2>{title}</h2>{/if}
				<div class="actions">
					{#if actions}{@render actions()}{/if}
					<CloseButton label={closeLabel} onclick={onclose} />
				</div>
			</header>
			<div class="body">{@render detail()}</div>
		</aside>
	{/if}
</div>

{#if !wide}
	<Sheet bind:open={sheetOpen} side="right" {title} {closeLabel} onclose={() => open && onclose()}>
		{#if actions}<div class="sheet-actions">{@render actions()}</div>{/if}
		{@render detail()}
	</Sheet>
{/if}

<style>
	.sc-split {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		gap: var(--space-1);
		align-items: start;
	}
	.sc-split.with-detail {
		grid-template-columns: minmax(0, 1fr) minmax(22rem, 34%);
	}
	.list {
		min-width: 0;
	}
	.detail {
		position: sticky;
		top: var(--split-view-top, 0);
		max-height: calc(100dvh - var(--split-view-top, 0px) - var(--split-view-bottom, var(--space-1)));
		display: flex;
		flex-direction: column;
		border: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
	}
	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-05);
		padding: var(--space-05) var(--space-075);
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	h2 {
		margin: 0;
		font-size: var(--type-label-size);
		line-height: var(--type-label-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		overflow-wrap: anywhere;
	}
	.actions {
		display: flex;
		align-items: center;
		gap: var(--space-05);
	}
	.body {
		padding: var(--space-075);
		overflow: auto;
	}
	.sheet-actions {
		display: flex;
		gap: var(--space-05);
		margin-bottom: var(--space-075);
	}
</style>
