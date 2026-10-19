<script lang="ts">
	import Icon from './icon.svelte';

	/**
	 * A folded region (agents/DOCKING.md): a strip as wide as its round unfold button + 0.5rem either side, with the button (chevrons pointing into the page) at the
	 * top, then the region's icon and its label written vertically (the sidebar: where the user is; the script outline:
	 * "Outline"; the right dock: the open panel). The AppShell's bottom region (`side="bottom"`): a horizontal strip as
	 * thick as a rail is wide – the button (chevrons-up) 1rem from the left, then the icon and the label in one row. A
	 * click anywhere on the strip unfolds it; the button is the keyboard's way in. Dragging out of the rail (its Divider)
	 * opens it too.
	 */
	let {
		side,
		label = '',
		icon,
		unfoldLabel,
		title = unfoldLabel,
		controls,
		onunfold,
		testid
	}: {
		/** where the region is docked (bottom: a horizontal strip – the button, the icon, the label in one row) */
		side: 'left' | 'right' | 'bottom';
		label?: string;
		icon?: string;
		unfoldLabel: string;
		title?: string;
		controls?: string;
		onunfold: () => void;
		testid?: string;
	} = $props();
</script>

<!-- the strip is a big click target for the mouse; keyboard and screen readers use its button -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="sc-rail" class:bottom={side === 'bottom'} onclick={onunfold} data-testid={testid}>
	<button
		type="button"
		class="sc-round"
		onclick={(e) => (e.stopPropagation(), onunfold())}
		aria-label={unfoldLabel}
		{title}
		aria-expanded="false"
		aria-controls={controls}
	>
		<Icon name={side === 'left' ? 'chevrons-right' : side === 'bottom' ? 'chevrons-up' : 'chevrons-left'} />
	</button>
	{#if icon}<span class="sc-rail-icon"><Icon name={icon} /></span>{/if}
	{#if label}<span class="sc-rail-label">{label}</span>{/if}
</div>

<style>
	.sc-rail {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.75rem;
		box-sizing: border-box;
		height: 100%;
		min-height: 0;
		/* the round button at the height of the region headers' buttons (the dock panels' – one control row with
		   var(--space-05) above it; owner 2026-10-07, 10.57) */
		padding-top: var(--space-05);
		overflow: hidden;
		cursor: pointer;
	}
	.sc-rail-icon {
		display: flex;
		color: var(--text-muted);
	}
	.sc-rail-label {
		writing-mode: vertical-rl;
		transform: rotate(180deg);
		color: var(--text-muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		max-height: calc(100% - 6rem);
	}
	/* a bottom region's rail: one row, the button at the left (1rem like a panel's content), the label written across */
	.sc-rail.bottom {
		flex-direction: row;
		height: auto;
		padding: var(--space-05) var(--space-1);
	}
	.bottom .sc-rail-label {
		writing-mode: horizontal-tb;
		transform: none;
		max-height: none;
		min-width: 0;
	}
</style>
