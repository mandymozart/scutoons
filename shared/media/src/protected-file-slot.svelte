<script lang="ts">
	import { CloseIcon, Icon } from '@scutoons/ui';
	import { mediaAssets, mediaText } from './setup.ts';

	/**
	 * A score's file as a slot (agents/research/SCORES.md §5): the button uploads into the PROTECTED storage
	 * (PUT /api/assets/protected – never the public one, not in the asset picker) and gives back its path
	 * `scores/<item>/<hash>.<ext>`; × clears it. Its name is the button, the path the tooltip, empty = a dashed slot.
	 * Needs the item's id (a new item: save it first).
	 */
	let { item, file, value, label, onchange, testid }: {
		/** the catalog item id (the folder) */
		item: string | null | undefined;
		file: 'musicxml' | 'pdf';
		value: string | null | undefined;
		label: string;
		onchange: (path: string | null) => void;
		testid?: string;
	} = $props();

	const t = $derived(mediaText());
	let input = $state<HTMLInputElement>();
	let busy = $state(false);
	let error = $state<string | null>(null);
	const name = $derived(value ? value.split('/').pop() : '');

	async function upload(e: Event) {
		const chosen = (e.currentTarget as HTMLInputElement).files?.[0];
		(e.currentTarget as HTMLInputElement).value = '';
		if (!chosen || !item) return;
		busy = true;
		error = null;
		try {
			const result = await mediaAssets().uploadProtected(chosen, item, file);
			onchange(result.path);
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		} finally {
			busy = false;
		}
	}
</script>

<div class="protected-slot" data-testid={testid}>
	<span class="sc-label">{label}</span>
	<div class="row">
		<button type="button" class="slot" class:empty={!value} disabled={!item || busy} onclick={() => input?.click()} title={value ?? t.chooseHint} aria-label="{label}: {value ?? t.choose}">
			{#if busy}<Icon name="loader" />{:else if value}<Icon name="lock" /> <span class="name">{name}</span>{:else}<Icon name="plus" />{/if}
		</button>
		{#if value}
			<button type="button" class="sc-round" aria-label={t.remove} title={t.remove} onclick={() => onchange(null)}><CloseIcon /></button>
		{/if}
	</div>
	<input bind:this={input} type="file" hidden accept={file === 'pdf' ? '.pdf,application/pdf' : '.musicxml,.mxl,.xml'} onchange={upload} />
	{#if error}<p class="sc-error">{error}</p>{/if}
</div>

<style>
	.protected-slot {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
	}
	.row {
		display: flex;
		align-items: center;
		gap: var(--space-05);
	}
	.slot {
		display: inline-flex;
		align-items: center;
		gap: var(--space-05);
		min-width: 0;
		max-width: 100%;
		height: var(--control-height);
		padding: 0 var(--space-075);
		background: var(--paper);
		color: var(--ink);
		border: var(--border-width-regular) solid var(--ink);
		border-radius: var(--radius-none);
		font: inherit;
		cursor: pointer;
	}
	.slot.empty {
		border-style: dashed;
		justify-content: center;
		width: var(--control-height);
		padding: 0;
	}
	.slot:disabled {
		cursor: default;
		color: var(--text-muted);
	}
	.name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
