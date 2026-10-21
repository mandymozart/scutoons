<script lang="ts" module>
	import type { Snippet } from 'svelte';

	/** one cell of the phone bar: an action (`run`), a group (`items`: its own row, ‹ back), or a custom cell (`content`) */
	export interface PhoneBarItem {
		id: string;
		/** a Feather icon name */
		icon?: string;
		/** the small word under the icon; the accessible name + tooltip */
		label: string;
		run?: () => void;
		/** a group: a tap shows these instead of the main row (‹ goes back) */
		items?: PhoneBarItem[];
		/** a custom cell (a language switch, a status chip, a note field …) */
		content?: Snippet;
		disabled?: boolean;
		/** pressed / on (a toggle, an open panel) */
		active?: boolean;
		/** the primary action: filled (save while there is something to save) */
		primary?: boolean;
		/** a small mark on the icon (a count, a dot) */
		badge?: string | number;
		/** a group stays open after one of its actions ran (undo / redo: tapped repeatedly) */
		stay?: boolean;
		testid?: string;
	}
</script>

<script lang="ts">
	import Icon from './icon.svelte';

	/**
	 * Phones (UI-CONCEPTS E.10, owner 2026-10-11 "an CapCut anlehnen"): the editors' one bottom bar. The main row holds a
	 * few entries – icon + a small word under it (the word for testing; maybe dropped once it is intuitive); a group
	 * replaces the row with its own actions (‹ back at the left). With a `selection` (a text box, an item, a place …)
	 * the row shows that object's own actions instead – ‹ deselects – like CapCut's clip toolbar. A row that does not
	 * fit scrolls sideways; a white fade at an edge shows that more follows there. Rendered in the ActionBar's `phone`.
	 * Desktop too (owner 2026-10-12, "phone grouping" – the same groups at every width): from 768px the cells sit in a
	 * row at the bar's end, icon + word side by side at the control height; the selection's row is a phone thing only
	 * (a desktop has the selection's panel beside the canvas, and Save / Publish stay in reach).
	 */
	let {
		items,
		selection = null,
		label = 'Actions',
		backLabel = 'Back'
	}: {
		items: PhoneBarItem[];
		/** the selected object's row; `onback` deselects */
		selection?: { label: string; items: PhoneBarItem[]; onback: () => void } | null;
		label?: string;
		backLabel?: string;
	} = $props();

	let phone = $state(true);
	$effect(() => {
		const query = window.matchMedia('(max-width: 767px)');
		phone = query.matches;
		const change = (event: MediaQueryListEvent) => (phone = event.matches);
		query.addEventListener('change', change);
		return () => query.removeEventListener('change', change);
	});
	const picked = $derived(phone ? selection : null);

	let openGroup = $state<string | null>(null);
	const group = $derived(openGroup ? items.find((i) => i.id === openGroup && i.items) : undefined);
	// a new selection closes an open group
	$effect(() => {
		if (picked) openGroup = null;
	});
	const row = $derived(picked ? picked.items : group ? (group.items ?? []) : items);
	const back = $derived(picked ? picked.onback : group ? () => (openGroup = null) : null);

	function tap(item: PhoneBarItem) {
		if (item.items) {
			openGroup = item.id;
			return;
		}
		item.run?.();
		if (group && !item.stay) openGroup = null;
	}

	// the faded edges: more to scroll to on that side
	let scroller = $state<HTMLElement>();
	let fadeStart = $state(false);
	let fadeEnd = $state(false);
	function edges() {
		if (!scroller) return;
		fadeStart = scroller.scrollLeft > 2;
		fadeEnd = scroller.scrollLeft + scroller.clientWidth < scroller.scrollWidth - 2;
	}
	$effect(() => {
		const el = scroller;
		if (!el) return;
		void row;
		el.scrollLeft = 0;
		edges();
		const observer = new ResizeObserver(edges);
		observer.observe(el);
		const mutations = new MutationObserver(edges);
		mutations.observe(el, { childList: true, subtree: true });
		return () => {
			observer.disconnect();
			mutations.disconnect();
		};
	});
</script>

<div class="phone-bar" role="toolbar" aria-label={picked ? picked.label : group ? group.label : label} data-testid="phone-bar">
	{#if back}
		<button type="button" class="back" aria-label={backLabel} title={backLabel} onclick={back} data-testid="phone-bar-back"><Icon name="chevron-left" /></button>
	{/if}
	<div class="wrap" class:fade-start={fadeStart} class:fade-end={fadeEnd}>
		<div class="row" bind:this={scroller} onscroll={edges}>
			{#each row as item (item.id)}
				{#if item.content}
					<div class="cell custom" data-testid={item.testid ?? `phone-bar-${item.id}`}>{@render item.content()}</div>
				{:else}
					<button
						type="button"
						class="cell"
						class:primary={item.primary}
						class:active={item.active}
						disabled={item.disabled}
						aria-pressed={item.active ?? undefined}
						aria-haspopup={item.items ? 'true' : undefined}
						title={item.label}
						aria-label={item.label}
						onclick={() => tap(item)}
						data-testid={item.testid ?? `phone-bar-${item.id}`}
					>
						<span class="icon">{#if item.icon}<Icon name={item.icon} />{/if}{#if item.badge !== undefined && item.badge !== ''}<span class="badge">{item.badge}</span>{/if}</span>
						<span class="word">{item.label}</span>
					</button>
				{/if}
			{/each}
		</div>
	</div>
</div>

<style>
	.phone-bar {
		display: flex;
		align-items: stretch;
		width: 100%;
		min-width: 0;
		height: 100%;
	}
	.back {
		all: unset;
		box-sizing: border-box;
		flex: none;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 2.5rem;
		border-right: var(--border-width-regular) solid var(--ink);
		color: var(--ink);
		cursor: pointer;
	}
	.wrap {
		position: relative;
		flex: 1;
		min-width: 0;
		display: flex;
	}
	.wrap::before,
	.wrap::after {
		content: '';
		position: absolute;
		z-index: 1;
		top: 0;
		bottom: 0;
		width: 2.5rem;
		pointer-events: none;
		opacity: 0;
		transition: opacity 0.15s;
	}
	.wrap::before {
		left: 0;
		background: linear-gradient(to left, transparent, var(--paper));
	}
	.wrap::after {
		right: 0;
		background: linear-gradient(to right, transparent, var(--paper));
	}
	.wrap.fade-start::before,
	.wrap.fade-end::after {
		opacity: 1;
	}
	.row {
		flex: 1;
		min-width: 0;
		display: flex;
		align-items: stretch;
		overflow-x: auto;
		scrollbar-width: none;
		overscroll-behavior-x: contain;
	}
	.row::-webkit-scrollbar {
		display: none;
	}
	.cell {
		all: unset;
		box-sizing: border-box;
		flex: 1 0 auto;
		min-width: 3.75rem;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.1rem;
		padding: 0 0.35rem;
		color: var(--ink);
		cursor: pointer;
	}
	.cell.custom {
		flex-direction: row;
		cursor: default;
	}
	.cell:disabled {
		opacity: 0.35;
		cursor: default;
	}
	.cell.primary:not(:disabled) {
		background: var(--ink);
		color: var(--paper);
	}
	.cell.active {
		box-shadow: inset 0 -0.1875rem 0 var(--ink);
	}
	.cell:focus-visible,
	.back:focus-visible {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-focus));
	}
	.icon {
		position: relative;
		display: inline-flex;
	}
	.badge {
		position: absolute;
		top: -0.35rem;
		right: -0.7rem;
		min-width: 0.9rem;
		padding: 0 0.15rem;
		box-sizing: border-box;
		border-radius: 999rem;
		background: var(--ink);
		color: var(--paper);
		font-size: 0.6rem;
		line-height: 0.9rem;
		text-align: center;
	}
	.cell.primary .badge {
		background: var(--paper);
		color: var(--ink);
	}
	/* phones: icons only (owner 2026-10-21: "i dont want the labels there") – the word is the tooltip and the aria-label */
	.word {
		display: none;
		white-space: nowrap;
	}
	/* desktop: a row of cells at the bar's end – icon + word side by side, the control height, square (grouped controls) */
	@media (min-width: 768px) {
		.phone-bar {
			justify-content: flex-end;
			align-items: center;
			gap: var(--space-05);
		}
		.wrap {
			flex: 0 1 auto;
		}
		.row {
			gap: var(--space-025);
			align-items: center;
		}
		.cell,
		.back {
			flex: 0 0 auto;
			flex-direction: row;
			gap: var(--space-05);
			min-width: var(--control-height);
			min-height: var(--control-height);
			padding: 0 var(--space-075);
			border: var(--border-width-regular) solid transparent;
		}
		.back {
			width: auto;
			border-color: var(--border-muted);
		}
		.cell:hover:not(:disabled),
		.back:hover {
			border-color: var(--ink);
		}
		.cell.custom {
			padding: 0;
		}
		.cell.active {
			box-shadow: none;
			border-color: var(--ink);
		}
		.word {
			display: inline;
			font-size: var(--type-body-size);
			line-height: var(--type-button-line);
		}
		.badge {
			top: -0.45rem;
			right: -0.6rem;
		}
	}
</style>
