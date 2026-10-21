<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import { asset } from '$lib/utils/cdn.ts';
	import { edgeFade } from '$lib/editor/edge-fade.ts';

	/**
	 * One row of faces with their words – the personas or the looks of the character page (research/CHARACTER-PAGE.md E:
	 * "the same row for persona and look"). It scrolls sideways, edge to edge between the page's dividers, its padding
	 * inside; a fade at the edge that has more (owner 2026-10-20: "that transparent gradient overlay"). The picked item is
	 * inverted; `ringed` rings it too (the level the inspector shows, K3). `+` at the end turns into a name field.
	 */
	let {
		items,
		value,
		label,
		ringed = false,
		addLabel,
		onpick,
		onadd,
		testid
	}: {
		items: { id: string; label: string; src?: string; count?: string; warning?: boolean }[];
		value: string;
		label: string;
		ringed?: boolean;
		/** the add field's placeholder (the + only with `onadd`) */
		addLabel?: string;
		onpick: (id: string) => void;
		onadd?: (label: string) => void;
		testid: string;
	} = $props();

	let scroller = $state<HTMLElement>();
	let adding = $state(false);
	let name = $state('');
	let input = $state<HTMLInputElement>();

	// the picked one comes into view (a pick from elsewhere: the URL, a new persona)
	$effect(() => {
		const el = scroller?.querySelector<HTMLElement>(`[data-id="${CSS.escape(value)}"]`);
		el?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
	});
	function startAdd() {
		adding = true;
		name = '';
		queueMicrotask(() => input?.focus());
	}
	function add() {
		const n = name.trim();
		if (!n) return;
		onadd?.(n);
		adding = false;
	}
</script>

<div class="face-row" bind:this={scroller} use:edgeFade role="group" aria-label={label} data-testid={testid}>
	{#each items as item (item.id)}
		<button
			type="button"
			class="face"
			class:on={item.id === value}
			class:ringed={ringed && item.id === value}
			class:warning={item.warning}
			aria-pressed={item.id === value}
			data-id={item.id}
			data-control-exempt="a face row: picture + word"
			onclick={() => onpick(item.id)}
			data-testid="{testid}-{item.id}"
		>
			{#if item.src}<img src={asset(item.src)} alt="" loading="lazy" />{:else}<span class="noimg"></span>{/if}
			<span class="word">{item.label}</span>
			{#if item.count}<span class="count">{item.count}</span>{/if}
		</button>
	{/each}
	{#if onadd}
		{#if adding}
			<span class="add-field">
				<input bind:this={input} type="text" bind:value={name} placeholder={addLabel} aria-label={addLabel} onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); add(); } else if (e.key === 'Escape') adding = false; }} onblur={() => !name.trim() && (adding = false)} data-testid="{testid}-new" />
			</span>
		{:else}
			<button type="button" class="plus" onclick={startAdd} aria-label={addLabel} title={addLabel} data-testid="{testid}-add"><Icon name="plus" /></button>
		{/if}
	{/if}
</div>

<style>
	.face-row {
		display: flex;
		gap: var(--space-025);
		align-items: center;
		overflow-x: auto;
		scrollbar-width: none;
		padding: var(--space-025) var(--space-1);
		min-width: 0;
		--fade: 2.5rem;
	}
	.face-row::-webkit-scrollbar {
		display: none;
	}
	.face-row:global([data-fade='r']) {
		mask-image: linear-gradient(to right, #000 calc(100% - var(--fade)), transparent);
	}
	.face-row:global([data-fade='l']) {
		mask-image: linear-gradient(to right, transparent, #000 var(--fade));
	}
	.face-row:global([data-fade='lr']) {
		mask-image: linear-gradient(to right, transparent, #000 var(--fade), #000 calc(100% - var(--fade)), transparent);
	}
	.face {
		all: unset;
		box-sizing: border-box;
		flex: none;
		display: inline-flex;
		align-items: center;
		gap: var(--space-05);
		height: var(--control-height);
		padding: 0 var(--space-05) 0 var(--space-025);
		border: var(--border-width-hairline) solid var(--border-muted);
		white-space: nowrap;
		cursor: pointer;
	}
	.face:hover,
	.face:focus-visible {
		border-color: var(--ink);
	}
	.face.on {
		background: var(--ink);
		color: var(--paper);
		border-color: var(--ink);
	}
	.face.ringed {
		outline: calc(2 * var(--border-width-regular)) solid var(--ink);
		outline-offset: var(--border-width-regular);
	}
	.face.warning:not(.on) {
		border-style: dashed;
		border-color: var(--state-error);
	}
	.face img,
	.noimg {
		width: calc(var(--control-height) - 0.5rem);
		height: calc(var(--control-height) - 0.5rem);
		object-fit: cover;
		flex: none;
		background: var(--surface-hover);
	}
	.count {
		font-size: var(--type-caption-size);
		opacity: 0.7;
	}
	.plus {
		all: unset;
		box-sizing: border-box;
		flex: none;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: var(--control-height);
		height: var(--control-height);
		border: var(--border-width-hairline) dashed var(--border-muted);
		color: var(--text-muted);
		cursor: pointer;
	}
	.plus:hover,
	.plus:focus-visible {
		border-color: var(--ink);
		color: var(--ink);
	}
	.add-field input {
		width: 12rem;
	}
	/* the ring sits outside the button: room above / below in the row */
	@media (pointer: coarse) {
		.face,
		.plus {
			height: max(2.75rem, var(--control-height));
		}
	}
</style>
