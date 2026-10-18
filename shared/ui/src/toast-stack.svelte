<script lang="ts">
	import type { Toaster } from './toasts.svelte.ts';
	import CloseButton from './close-button.svelte';

	/**
	 * Renders a toaster's toasts, fixed bottom-left (owner 2026-10-07: the left side is used less; bottom-right covered fields) – above the ActionBar when the page has one (never in the page
	 * flow). Errors are announced assertively. A toast's `action` (e.g. Undo) is a button before the close cross.
	 */
	let { toaster, dismissLabel = 'Dismiss' }: { toaster: Toaster; dismissLabel?: string } = $props();
</script>

<div class="sc-toast-stack" aria-live="polite">
	{#each toaster.items as toast (toast.id)}
		<div class="toast {toast.type}" role={toast.type === 'error' ? 'alert' : 'status'}>
			<p>{toast.message}</p>
			{#if toast.action}<button type="button" class="secondary action" onclick={() => toaster.act(toast.id)} data-testid="toast-action">{toast.action.label}</button>{/if}
			<CloseButton label={dismissLabel} size="var(--type-body-size, 1rem)" onclick={() => toaster.dismiss(toast.id)} />
		</div>
	{/each}
</div>

<style>
	.sc-toast-stack {
		position: fixed;
		left: var(--space-1);
		bottom: var(--space-1);
		z-index: 1000;
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: var(--space-05);
		max-width: min(24rem, calc(100vw - 2rem));
		pointer-events: none;
	}
	/* a page with the fixed bottom ActionBar: the stack sits right above it (and above a bar a page puts on it:
	   `--sc-above-action-bar`, e.g. the creator's slice bar on phones) */
	:global(body:has(.sc-action-bar)) .sc-toast-stack {
		bottom: calc(var(--action-bar-height, var(--tool-bar-height)) + var(--sc-above-action-bar, 0rem) + var(--space-1));
	}
	/* phones: while the ActionBar's "More" sheet is open the toasts wait under its scrim (they covered its entries) */
	@media (max-width: 767px) {
		:global(body:has(.sc-action-bar .more-panel)) .sc-toast-stack {
			z-index: 30;
		}
	}
	.toast {
		display: flex;
		align-items: flex-start;
		gap: var(--space-075);
		box-sizing: border-box;
		padding: var(--space-075) var(--space-1);
		background: var(--paper);
		color: var(--ink);
		border: var(--border-width-regular) solid var(--ink);
		border-radius: var(--radius-none);
		pointer-events: auto;
		max-width: 100%;
	}
	.success {
		border-left: 0.5em solid var(--primary-cyan);
	}
	.warning {
		border-left: 0.5em solid var(--state-warning);
	}
	.error {
		border-left: 0.5em solid var(--state-error);
	}
	.action {
		flex: none;
		align-self: center;
	}
	/* a long unbroken word (a JSON path, a pattern in a save error) wraps anywhere – else the toast grew past the screen
	   and pushed its close cross out of reach (owner 2026-10-18) */
	p {
		flex: 1 1 auto;
		min-width: 0;
		margin: 0;
		align-self: center;
		line-height: var(--type-body-line);
		overflow-wrap: anywhere;
	}
	.toast :global(.sc-close-button) {
		flex: none;
	}
	/* the close cross level with the first line, the control's extra height hangs into the padding */
	.toast :global(.sc-close-button) {
		margin-block: calc((var(--type-body-line) * var(--type-body-size) - var(--control-height)) / 2);
		margin-inline-end: calc(-1 * var(--space-05));
	}
</style>
