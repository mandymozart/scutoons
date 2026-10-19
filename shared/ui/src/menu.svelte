<script lang="ts">
	import type { Snippet } from 'svelte';
	import { clickOutside } from './click-outside.ts';

	/**
	 * Dropdown menu (disclosure pattern): a trigger button and a popup with the app's items (buttons / links).
	 * Closes on outside click, Escape (focus returns to the trigger) and after an item is chosen. `drop="up"` opens it
	 * above the trigger (a menu in the bottom ActionBar). The popup is as wide as its widest item (12rem … 40rem – not the
	 * trigger's width: the studio's notes read one word per line), moved back inside the window when it would stick out;
	 * its items are left-aligned whatever an app's button rule centres.
	 */
	let {
		label,
		trigger,
		align = 'start',
		drop = 'down',
		open = $bindable(false),
		children
	}: {
		/** trigger text, also the accessible name */
		label: string;
		/** custom trigger content (the button stays) */
		trigger?: Snippet;
		align?: 'start' | 'end';
		drop?: 'down' | 'up';
		open?: boolean;
		children: Snippet;
	} = $props();

	const uid = $props.id();
	let button: HTMLButtonElement | null = $state(null);
	let popup: HTMLElement | null = $state(null);
	/** rem: how far the open popup moves to stay 0.5rem inside the window (measured once it shows) */
	let shift = $state(0);
	$effect(() => {
		if (!popup) return void (shift = 0);
		const [r, rem] = [popup.getBoundingClientRect(), parseFloat(getComputedStyle(document.documentElement).fontSize) || 16];
		const [lo, hi] = [rem / 2, window.innerWidth - rem / 2];
		shift = (r.left < lo ? lo - r.left : r.right > hi ? Math.max(lo - r.left, hi - r.right) : 0) / rem;
	});

	function close(event?: Event) {
		if (!open) return;
		open = false;
		if (event instanceof KeyboardEvent) button?.focus();
	}

	function onPopupClick(event: MouseEvent) {
		if ((event.target as HTMLElement).closest('a, button')) close();
	}
</script>

<div class="sc-menu" {@attach clickOutside(close)}>
	<button
		bind:this={button}
		type="button"
		class="trigger"
		aria-haspopup="true"
		aria-expanded={open}
		aria-controls="{uid}-menu"
		aria-label={trigger ? label : undefined}
		onclick={() => (open = !open)}
	>
		{#if trigger}{@render trigger()}{:else}{label}{/if}
	</button>
	{#if open}
		<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
		<div class="popup align-{align}" class:up={drop === 'up'} id="{uid}-menu" bind:this={popup} style:translate={shift ? `${shift}rem 0` : null} onclick={onPopupClick}>
			{@render children()}
		</div>
	{/if}
</div>

<style>
	.sc-menu {
		position: relative;
		display: inline-block;
	}
	.trigger {
		box-sizing: border-box;
		min-height: var(--control-height);
		margin: 0;
		padding: 0 var(--space-1);
		background: var(--paper);
		color: var(--ink);
		border: var(--border-width-regular) solid var(--border-muted);
		border-radius: var(--radius-none);
		font: inherit;
		line-height: var(--type-button-line);
		cursor: pointer;
	}
	.trigger:hover,
	.trigger[aria-expanded='true'] {
		border-color: var(--ink);
	}
	.trigger:focus-visible {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: var(--focus-offset);
	}
	.popup {
		position: absolute;
		top: calc(100% + var(--space-025));
		z-index: 100;
		display: flex;
		flex-direction: column;
		/* its widest item, not the trigger's width (an absolute box shrinks to its trigger-wide container) */
		width: max-content;
		min-width: 12rem;
		max-width: min(40rem, calc(100vw - 1rem));
		padding: var(--space-025) 0;
		background: var(--paper);
		border: var(--border-width-regular) solid var(--ink);
		border-radius: var(--radius-none);
	}
	.popup.up {
		top: auto;
		bottom: calc(100% + var(--space-025));
	}
	.align-start {
		left: 0;
	}
	.align-end {
		right: 0;
	}
	.popup :global(:is(a, button)) {
		display: flex;
		align-items: center;
		/* left-aligned (an app's button rule may centre – the studio's buttons.css) */
		justify-content: flex-start;
		gap: var(--space-05);
		box-sizing: border-box;
		width: 100%;
		min-height: var(--control-height);
		margin: 0;
		padding: 0 var(--space-1);
		background: transparent;
		color: var(--ink);
		border: 0;
		border-radius: var(--radius-none);
		font: inherit;
		line-height: 1.2;
		text-align: left;
		text-decoration: none;
		cursor: pointer;
	}
	/* hover frames, focus = the 3px ring (DESIGN.md "Selection": hover is never inverted) – inside the popup's frame */
	.popup :global(:is(a, button):hover) {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-regular));
	}
	.popup :global(:is(a, button):focus-visible) {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-focus));
	}
</style>
