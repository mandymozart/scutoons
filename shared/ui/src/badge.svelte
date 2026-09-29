<script lang="ts">
	import type { Snippet } from 'svelte';

	/**
	 * Status badge (claimed, revoked, draft, published, roles). Hairline border, caption size, square.
	 * tone: neutral (border-muted) · ink (filled ink) · accent (cyan fill) · warning · error (state-colored border, ink text).
	 */
	let {
		tone = 'neutral',
		children
	}: {
		tone?: 'neutral' | 'ink' | 'accent' | 'warning' | 'error';
		children: Snippet;
	} = $props();
</script>

<span class="sc-badge {tone}">{@render children()}</span>

<style>
	.sc-badge {
		display: inline-block;
		box-sizing: border-box;
		padding: 0.1em 0.5em;
		border: var(--border-width-hairline) solid var(--border-muted);
		border-radius: var(--radius-none);
		background: var(--paper);
		color: var(--ink);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		white-space: nowrap;
		vertical-align: middle;
	}
	.ink {
		background: var(--ink);
		color: var(--paper);
		border-color: var(--ink);
	}
	.accent {
		background: var(--primary-cyan);
		border-color: var(--ink);
	}
	/* state colors on borders only: as text they are below 4.5:1 on paper */
	.warning {
		border-color: var(--state-warning);
		border-left-width: var(--border-width-regular);
	}
	.error {
		border-color: var(--state-error);
		border-left-width: var(--border-width-regular);
	}
</style>
