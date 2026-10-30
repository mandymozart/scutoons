<script lang="ts">
	import { tick } from 'svelte';
	import { dialogState } from './dialogs.svelte.ts';

	/** Shows the in-page dialogs of `confirmDialog` / `askDialog` (dialogs.svelte.ts); once in the app's root layout. */
	let { cancelLabel = 'Cancel' }: { cancelLabel?: string } = $props();

	let element = $state<HTMLDialogElement>();
	let text = $state('');
	/** the ticked lines of a checklist */
	let picked = $state<Set<string>>(new Set());
	const request = $derived(dialogState.current);
	const allIds = $derived(request?.checklist ? request.checklist.groups.flatMap((g) => g.items.map((i) => i.id)) : []);

	$effect(() => {
		if (!element) return;
		if (request && !element.open) {
			text = request.input?.value ?? '';
			picked = new Set(request.checklist ? request.checklist.groups.flatMap((g) => g.items.filter((i) => i.checked !== false).map((i) => i.id)) : []);
			element.showModal();
			// a suggestion (askDialog `select`): selected once the field shows it – typing replaces it, Enter takes it
			if (request.input?.select) void tick().then(() => element?.querySelector<HTMLInputElement>('#sc-dialog-input')?.select());
		} else if (!request && element.open) {
			element.close();
		}
	});

	function confirm(event: SubmitEvent) {
		event.preventDefault();
		if (request?.checklist) return dialogState.close(allIds.filter((id) => picked.has(id)));
		dialogState.close(request?.input ? text.trim() : true);
	}
	function toggle(id: string, on: boolean) {
		const next = new Set(picked);
		if (on) next.add(id);
		else next.delete(id);
		picked = next;
	}
</script>

<dialog bind:this={element} class="sc-dialog" oncancel={(e) => { e.preventDefault(); dialogState.close(request?.input ? null : false); }} aria-labelledby="sc-dialog-title">
	{#if request}
		<form method="dialog" onsubmit={confirm}>
			<h2 id="sc-dialog-title">{request.title}</h2>
			{#if request.message}<p class="message">{request.message}</p>{/if}
			{#if request.checklist}
				<div class="checklist">
					{#each request.checklist.groups as group, g (g)}
						{#if group.label}<p class="group">{group.label}</p>{/if}
						{#each group.items as item (item.id)}
							<label class="line">
								<input type="checkbox" checked={picked.has(item.id)} onchange={(e) => toggle(item.id, e.currentTarget.checked)} />
								<span class="text"><span>{item.label}</span>{#if item.detail}<small>{item.detail}</small>{/if}</span>
							</label>
						{/each}
					{/each}
				</div>
				<div class="all-none sc-seg" role="group">
					<button type="button" onclick={() => (picked = new Set(allIds))}>{request.checklist.allLabel}</button>
					<button type="button" onclick={() => (picked = new Set())}>{request.checklist.noneLabel}</button>
				</div>
			{/if}
			{#if request.choices}
				<div class="choices">
					{#each request.choices as item (item.id)}
						<button type="button" class="secondary choice" onclick={() => dialogState.close(item.id)}>
							<span>{item.label}</span>{#if item.detail}<small>{item.detail}</small>{/if}
						</button>
					{/each}
				</div>
			{/if}
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
				{#if !request.choices}<button type="submit" class="primary" autofocus={!request.input && !request.checklist} disabled={!!request.checklist && picked.size === 0}>{request.checklist ? request.checklist.countLabel(picked.size) : request.input?.skipLabel && !text.trim() ? request.input.skipLabel : request.confirmLabel}</button>{/if}
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
	/* the text field only (askDialog) – a checklist's boxes keep their own size */
	#sc-dialog-input {
		box-sizing: border-box;
		width: 100%;
	}
	.line input {
		flex: none;
	}
	/* a choice list (choiceDialog): one line per option, a tap picks it */
	.choices {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		max-height: min(50vh, 24rem);
		overflow: auto;
	}
	.choice {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 0.1rem;
		text-align: left;
		height: auto;
		min-height: var(--control-height);
		padding: 0.35rem 0.75rem; /* control-ok: a two-line choice row */
	}
	.choice small {
		color: var(--text-muted);
		font-size: var(--type-caption-size);
	}
	.buttons {
		display: flex;
		flex-wrap: wrap;
		justify-content: flex-end;
		gap: 0.5rem;
	}
	/* a checklist: scrolls when long; one line per file, its detail quieter below */
	.checklist {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		max-height: min(50vh, 24rem);
		overflow: auto;
	}
	.group {
		margin: 0.4rem 0 0;
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--muted);
	}
	.line {
		display: flex;
		gap: 0.5rem;
		align-items: flex-start;
		cursor: pointer;
	}
	.line .text {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	.line small {
		color: var(--muted);
		font-size: var(--type-caption-size);
		overflow-wrap: anywhere;
	}
	.all-none {
		align-self: flex-start;
	}
	/* cancel on the left, the primary action on the right (owner 2026-10-06) */
	.cancel {
		margin-right: auto;
	}
	/* a phone: the two share the row in equal halves, a long label wraps – wrapped, the primary sat alone on a second
	   row, offset to the right (#194) */
	@media (max-width: 479px) {
		.buttons {
			flex-wrap: nowrap;
		}
		.buttons > button {
			flex: 1 1 0;
			min-width: 0;
			white-space: normal;
		}
		.cancel {
			margin-right: 0;
		}
	}
</style>
