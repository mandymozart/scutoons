<script lang="ts">
	import { dialogState } from './dialogs.svelte.ts';

	/** Shows the in-page dialogs of `confirmDialog` / `askDialog` (dialogs.svelte.ts); once in the app's root layout. */
	let { cancelLabel = 'Cancel' }: { cancelLabel?: string } = $props();

	let element = $state<HTMLDialogElement>();
	let text = $state('');
	const request = $derived(dialogState.current);

	$effect(() => {
		if (!element) return;
		if (request && !element.open) {
			text = '';
			element.showModal();
		} else if (!request && element.open) {
			element.close();
		}
	});

	function confirm(event: SubmitEvent) {
		event.preventDefault();
		dialogState.close(request?.input ? text.trim() : true);
	}
</script>

<dialog bind:this={element} class="sc-dialog" oncancel={(e) => { e.preventDefault(); dialogState.close(request?.input ? null : false); }} aria-labelledby="sc-dialog-title">
	{#if request}
		<form method="dialog" onsubmit={confirm}>
			<h2 id="sc-dialog-title">{request.title}</h2>
			{#if request.message}<p class="message">{request.message}</p>{/if}
			{#if request.input}
				<label class="sc-label" for="sc-dialog-input">{request.input.label}</label>
				<!-- svelte-ignore a11y_autofocus -->
				<input id="sc-dialog-input" type="text" bind:value={text} placeholder={request.input.placeholder} autofocus />
			{/if}
			<div class="buttons">
				<button type="button" class="ghost cancel" onclick={() => dialogState.close(request?.input ? null : false)}>{request.cancelLabel ?? cancelLabel}</button>
				<!-- one primary action, last (owner 2026-10-06): with a skip label it says "save without note" while the field
				     is empty and "save" once something is typed – submitting empty is the skip -->
				<!-- svelte-ignore a11y_autofocus -->
				<button type="submit" class="primary" autofocus={!request.input}>{request.input?.skipLabel && !text.trim() ? request.input.skipLabel : request.confirmLabel}</button>
			</div>
		</form>
	{/if}
</dialog>

<style>
	.sc-dialog {
		box-sizing: border-box;
		width: min(32rem, calc(100vw - 2rem));
		padding: 1.25rem;
		color: var(--ink);
		background: var(--paper);
		border: var(--border-width-regular) solid var(--ink);
		border-radius: var(--radius-none);
	}
	.sc-dialog::backdrop {
		background: rgb(0 0 0 / 0.35);
	}
	form {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}
	h2 {
		margin: 0;
		font-size: var(--type-label-size);
		line-height: var(--type-label-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	.message {
		margin: 0;
		white-space: pre-line;
	}
	input {
		box-sizing: border-box;
		width: 100%;
	}
	.buttons {
		display: flex;
		flex-wrap: wrap;
		justify-content: flex-end;
		gap: 0.5rem;
	}
	/* cancel on the left, the primary action on the right (owner 2026-10-06) */
	.cancel {
		margin-right: auto;
	}
</style>
