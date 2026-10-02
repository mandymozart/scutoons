<script lang="ts">
	import type { Snippet } from 'svelte';
	import CloseButton from './close-button.svelte';

	/**
	 * Modal sheet on the native <dialog> (focus trap, Escape, top layer). `side`: bottom sheet (mobile reader
	 * pattern), right drawer (detail / triage view) or centered dialog. Square, 2px ink border, no shadow.
	 * Closes on Escape, backdrop click and the close button; `onclose` fires after every close.
	 */
	let {
		open = $bindable(false),
		title,
		side = 'center',
		closeLabel = 'Close',
		onclose,
		footer,
		children
	}: {
		open?: boolean;
		title?: string;
		side?: 'bottom' | 'right' | 'center';
		/** translated label of the close button */
		closeLabel?: string;
		onclose?: () => void;
		/** sticky action bar at the bottom */
		footer?: Snippet;
		children: Snippet;
	} = $props();

	let dialog: HTMLDialogElement | null = $state(null);

	$effect(() => {
		if (!dialog) return;
		if (open && !dialog.open) dialog.showModal();
		else if (!open && dialog.open) dialog.close();
	});

	function handleClose() {
		open = false;
		onclose?.();
	}

	function onBackdrop(event: MouseEvent) {
		// a click on the ::backdrop targets the dialog itself
		if (event.target === dialog) dialog?.close();
	}
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<dialog
	bind:this={dialog}
	class="sc-sheet side-{side}"
	aria-label={title}
	onclose={handleClose}
	onclick={onBackdrop}
>
	<div class="frame">
		<header>
			{#if title}<h2>{title}</h2>{/if}
			<CloseButton class="close" label={closeLabel} onclick={() => dialog?.close()} />
		</header>
		<div class="body">
			{@render children()}
		</div>
		{#if footer}
			<footer>{@render footer()}</footer>
		{/if}
	</div>
</dialog>

<style>
	.sc-sheet {
		box-sizing: border-box;
		padding: 0;
		border: var(--border-width-regular) solid var(--ink);
		border-radius: var(--radius-none);
		background: var(--paper);
		color: var(--ink);
		font-family: var(--font-mono);
		max-width: min(40rem, calc(100vw - 2rem));
		max-height: calc(100dvh - 2rem);
	}
	.sc-sheet::backdrop {
		background: rgb(0 0 0 / 0.4);
	}
	.side-bottom {
		width: 100%;
		max-width: 100%;
		margin: auto 0 0;
		border-width: var(--border-width-regular) 0 0;
	}
	.side-right {
		height: 100dvh;
		max-height: 100dvh;
		width: min(32rem, 100vw);
		margin: 0 0 0 auto;
		border-width: 0 0 0 var(--border-width-regular);
	}
	.frame {
		display: flex;
		flex-direction: column;
		max-height: inherit;
		height: 100%;
	}
	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-1);
		padding: var(--space-075) var(--space-1);
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	h2 {
		margin: 0;
		font-size: var(--type-label-size);
		line-height: var(--type-label-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	/* the close cross (CloseButton): right, level with the title */
	header :global(.close) {
		margin: 0 0 0 auto;
	}
	.body {
		padding: var(--space-1);
		/* dialogs scroll vertically only; content must wrap to the sheet width */
		overflow-x: hidden;
		overflow-y: auto;
		min-width: 0;
		flex: 1;
	}
	footer {
		position: sticky;
		bottom: 0;
		display: flex;
		justify-content: flex-end;
		gap: var(--space-05);
		padding: var(--space-075) var(--space-1);
		background: var(--paper);
		border-top: var(--border-width-regular) solid var(--ink);
	}
</style>
