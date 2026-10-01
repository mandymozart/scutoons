<script lang="ts">
	import type { Snippet } from 'svelte';
	import { clickOutside } from './click-outside.ts';

	/**
	 * Dropdown menu (disclosure pattern): a trigger button and a popup with the app's items (buttons / links).
	 * Closes on outside click, Escape (focus returns to the trigger) and after an item is chosen.
	 */
	let {
		label,
		trigger,
		align = 'start',
		open = $bindable(false),
		children
	}: {
		/** trigger text, also the accessible name */
		label: string;
		/** custom trigger content (the button stays) */
		trigger?: Snippet;
		align?: 'start' | 'end';
		open?: boolean;
		children: Snippet;
	} = $props();

	const uid = $props.id();
	let button: HTMLButtonElement | null = $state(null);

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
		<div class="popup align-{align}" id="{uid}-menu" onclick={onPopupClick}>
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
		padding: var(--space-025) var(--space-1);
		background: var(--paper);
		color: var(--ink);
		border: var(--border-width-regular) solid var(--ink);
		border-radius: var(--radius-none);
		font: inherit;
		line-height: 1.2;
		cursor: pointer;
	}
	.trigger:hover,
	.trigger[aria-expanded='true'] {
		background: var(--ink);
		color: var(--paper);
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
		min-width: 12rem;
		padding: var(--space-025) 0;
		background: var(--paper);
		border: var(--border-width-regular) solid var(--ink);
		border-radius: var(--radius-none);
	}
	.align-start {
		left: 0;
	}
	.align-end {
		right: 0;
	}
	.popup :global(:is(a, button)) {
		display: block;
		box-sizing: border-box;
		width: 100%;
		min-height: var(--control-height);
		margin: 0;
		padding: var(--space-05) var(--space-1);
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
	.popup :global(:is(a, button):hover),
	.popup :global(:is(a, button):focus-visible) {
		background: var(--ink);
		color: var(--paper);
		outline: none;
	}
</style>
