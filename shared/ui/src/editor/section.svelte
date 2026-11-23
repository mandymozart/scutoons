<script lang="ts">
	import type { Snippet } from 'svelte';
	import Icon from '../icon.svelte';
	import Toggle from './toggle.svelte';
	import { sections } from './sections.svelte.ts';

	/**
	 * A collapsible group of a panel's rows: the head (chevron, name, `aside` at the right) opens and closes it, kept per
	 * `id` in this browser (`open` = the default until toggled). Closed, `summary` follows the name ("bloom · strength
	 * 0.35"), so closed hides nothing that matters. A group switched as a whole passes `enabled` + `onenabled`: its head
	 * carries a `Toggle` that works without opening it (on opens it), the name dims while off. `nested`: a section inside
	 * another one – its name reads as a name, the line above it dotted.
	 */
	let {
		id,
		label,
		open = true,
		summary,
		enabled,
		onenabled,
		enabledLabel,
		nested = false,
		aside,
		children
	}: {
		id: string;
		label: string;
		open?: boolean;
		summary?: string;
		enabled?: boolean;
		onenabled?: (enabled: boolean) => void;
		enabledLabel?: string;
		nested?: boolean;
		aside?: Snippet;
		children: Snippet;
	} = $props();

	const isOpen = $derived(sections.isOpen(id, open));
	const switchable = $derived(enabled !== undefined && !!onenabled);
	const switchTo = (on: boolean) => {
		if (on) sections.setOpen(id, true);
		onenabled?.(on);
	};
</script>

<div class="section" class:nested class:off={switchable && !enabled} data-section={id}>
	<div class="head">
		<button type="button" class="toggle-open" aria-expanded={isOpen} onclick={() => sections.toggle(id, open)}>
			<Icon name={isOpen ? 'chevron-down' : 'chevron-right'} size="0.875rem" strokeWidth={2} />
			<span class="name">{label}</span>
			{#if !isOpen && summary}<span class="summary">· {summary}</span>{/if}
		</button>
		{#if switchable || aside}
			<span class="aside">
				{#if switchable}<Toggle on={!!enabled} label={enabledLabel ?? label} onchange={switchTo} />{/if}
				{@render aside?.()}
			</span>
		{/if}
	</div>
	{#if isOpen}<div class="rows">{@render children()}</div>{/if}
</div>

<style>
	.section {
		border-top: var(--editor-edge) solid var(--tool-line);
	}
	.section.nested {
		border-top-style: dotted;
	}
	.head {
		display: flex;
		align-items: center;
		gap: 0.375rem;
		padding: 0 var(--editor-pad) 0 calc(var(--editor-pad) - 0.25rem);
	}
	.toggle-open {
		display: inline-flex;
		align-items: center;
		justify-content: flex-start;
		gap: 0.375rem;
		flex: 1;
		min-width: 0;
		height: var(--control-height);
		padding: 0 0.25rem;
		border: none;
		border-radius: var(--editor-radius);
		background: none;
		color: var(--tool-muted);
		font: inherit;
		text-align: left;
		text-shadow: inherit;
		cursor: pointer;
	}
	.name {
		flex: none;
		font-size: var(--editor-label);
		letter-spacing: var(--editor-label-spacing);
		text-transform: uppercase;
	}
	.nested .name {
		color: var(--tool-text);
		font-size: inherit;
		letter-spacing: normal;
		text-transform: none;
	}
	.off .name {
		color: var(--tool-muted);
		opacity: 0.7;
	}
	.summary {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--editor-small);
	}
	/* open: the head on the head ground, its chevron in the text colour; hover: the hover ground (kit.css --kit-section-*) */
	.head:has(.toggle-open[aria-expanded='true']) {
		background: var(--kit-section-on-ground);
	}
	.toggle-open :global(svg) {
		color: var(--kit-section-rest-mark);
	}
	.toggle-open[aria-expanded='true'] :global(svg) {
		color: var(--kit-section-on-mark);
	}
	.toggle-open:focus-visible {
		outline: 2px solid var(--kit-focus-rest-ring);
		outline-offset: -2px;
	}
	@media (hover: hover) {
		.toggle-open:hover {
			background: var(--kit-section-hover-ground);
			color: var(--tool-text);
		}
	}
	.aside {
		display: inline-flex;
		align-items: center;
		gap: 0.375rem;
		flex: none;
	}
	.rows {
		display: grid;
		gap: 0.375rem;
		padding: 0 var(--editor-pad) 0.75rem;
	}
</style>
