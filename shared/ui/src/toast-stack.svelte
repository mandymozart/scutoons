<script lang="ts">
	import type { Toaster } from './toasts.svelte.ts';
	import CloseButton from './close-button.svelte';

	/**
	 * Renders a toaster's toasts, fixed bottom-right – above the ActionBar when the page has one (never in the page
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
		right: var(--space-1);
		bottom: var(--space-1);
		z-index: 1000;
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: var(--space-05);
		max-width: min(24rem, calc(100vw - 2rem));
		pointer-events: none;
	}
	/* a page with the fixed bottom ActionBar: the stack sits right above it */
	:global(body:has(.sc-action-bar)) .sc-toast-stack {
		bottom: calc(var(--action-bar-height, var(--tool-bar-height)) + var(--space-1));
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
	p {
		margin: 0;
		align-self: center;
		line-height: var(--type-body-line);
	}
	/* the close cross level with the first line, the control's extra height hangs into the padding */
	.toast :global(.sc-close-button) {
		margin-block: calc((var(--type-body-line) * var(--type-body-size) - var(--control-height)) / 2);
		margin-inline-end: calc(-1 * var(--space-05));
	}
</style>
