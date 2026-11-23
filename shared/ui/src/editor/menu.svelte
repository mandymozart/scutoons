<script lang="ts">
	import type { Snippet } from 'svelte';
	import { clickOutside } from '../click-outside.ts';
	import IconButton from './icon-button.svelte';
	import MenuPanel from './menu-panel.svelte';

	/**
	 * A menu under its icon (deck-kit bar.rs: the menu, the tab list; controls.rs `actions`): the IconButton – its name the
	 * tooltip – opens a MenuPanel a little under the pill it sits in, at the pill's end (`align`); a press outside, Escape or
	 * `close()` shuts it. `beside` is a second panel beside the first (a sub menu), toward the open side. It lies over every
	 * panel (z-index 60).
	 */
	let {
		label,
		icon = 'menu',
		about,
		dot = false,
		align = 'end',
		width = '17rem',
		beside,
		besideWidth = '19rem',
		besideOpen = false,
		testid,
		menuTestid,
		besideTestid,
		open = $bindable(false),
		onclose,
		children
	}: {
		label: string;
		icon?: string;
		about?: string;
		dot?: boolean;
		align?: 'start' | 'end';
		width?: string;
		beside?: Snippet<[close: () => void]>;
		besideWidth?: string;
		besideOpen?: boolean;
		testid?: string;
		menuTestid?: string;
		besideTestid?: string;
		open?: boolean;
		onclose?: () => void;
		children: Snippet<[close: () => void]>;
	} = $props();

	const close = () => {
		if (!open) return;
		open = false;
		onclose?.();
	};
</script>

<svelte:window onkeydown={(e) => open && e.key === 'Escape' && close()} />

<div class="kit-menu" {@attach clickOutside(close)}>
	<IconButton {label} {icon} title={about ?? label} {dot} aria-expanded={open} aria-haspopup="menu" onclick={() => (open ? close() : (open = true))} data-testid={testid} />
	{#if open}
		<div class="drop {align}">
			<MenuPanel {width} data-testid={menuTestid ?? (testid ? `${testid}-menu` : undefined)}>{@render children(close)}</MenuPanel>
			{#if beside}<MenuPanel width={besideWidth} open={besideOpen} data-testid={besideTestid}>{@render beside(close)}</MenuPanel>{/if}
		</div>
	{/if}
</div>

<style>
	.kit-menu {
		position: relative;
		flex: none;
	}
	/* under the pill the button sits in (its inset, then deck-kit's 0.43rem), flush with the pill's outer end */
	.drop {
		position: absolute;
		z-index: 60;
		top: calc(100% + var(--kit-inset) + 0.43rem);
		display: flex;
		align-items: flex-start;
		gap: var(--kit-seam);
	}
	.end {
		right: calc(-1 * var(--kit-inset));
		flex-direction: row-reverse;
	}
	.start {
		left: calc(-1 * var(--kit-inset));
	}
</style>
