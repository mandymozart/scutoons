<script lang="ts">
	import type { Toaster } from './toasts.svelte.ts';

	/** Renders a toaster's toasts, fixed bottom-right. Errors are announced assertively. */
	let { toaster, dismissLabel = 'Dismiss' }: { toaster: Toaster; dismissLabel?: string } = $props();
</script>

<div class="sc-toast-stack" aria-live="polite">
	{#each toaster.items as toast (toast.id)}
		<div class="toast {toast.type}" role={toast.type === 'error' ? 'alert' : 'status'}>
			<p>{toast.message}</p>
			<button type="button" aria-label={dismissLabel} title={dismissLabel} onclick={() => toaster.dismiss(toast.id)}>
				×
			</button>
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
	p {
		margin: 0;
		line-height: var(--type-body-line);
	}
	button {
		margin: 0;
		padding: 0 var(--space-025);
		background: transparent;
		border: 0;
		color: var(--ink);
		font: inherit;
		font-size: 1.25em;
		line-height: 1;
		cursor: pointer;
	}
</style>
