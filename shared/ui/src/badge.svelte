<script lang="ts">
	import type { Snippet } from 'svelte';

	/**
	 * Status badge (claimed, revoked, draft, published, roles). Hairline border, caption size, square.
	 * tone: neutral (2px ink frame) · ink (filled ink) · accent (cyan fill: "live" states only) · warning · error
	 * (state-colored frame + thick left rule, ink text). Caption size, bold; no uppercase (badges carry ids and
	 * codes, which are case-sensitive). DESIGN.md "Brutalist tools".
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
		padding: 0.15em 0.5em;
		border: var(--border-width-regular) solid var(--ink);
		border-radius: var(--radius-none);
		background: var(--paper);
		color: var(--ink);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		font-weight: var(--font-weight-bold);
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
		border-left-width: 0.5em;
	}
	.error {
		border-color: var(--state-error);
		border-left-width: 0.5em;
	}
</style>
