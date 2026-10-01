<script lang="ts">
	import type { Snippet } from 'svelte';

	/**
	 * Status badge (claimed, revoked, draft, published, roles). Hairline border, caption size, square.
	 * FILLED (owner 2026-10-01), colours from tokens.css `--badge-*`: tone neutral · published (alias accent: live,
	 * signed) · draft · soon · info · warning · error; neutral (alias ink) is black. White text where it reaches 4.5:1,
	 * else ink. Caption size, bold; no uppercase (badges carry ids and codes, which are case-sensitive).
	 */
	let {
		tone = 'neutral',
		children
	}: {
		tone?: 'neutral' | 'ink' | 'accent' | 'published' | 'draft' | 'soon' | 'info' | 'warning' | 'error';
		children: Snippet;
	} = $props();
</script>

<span class="sc-badge {tone}">{@render children()}</span>

<style>
	.sc-badge {
		display: inline-block;
		box-sizing: border-box;
		padding: 0.15em 0.5em;
		border: 0;
		border-radius: var(--radius-none);
		background: var(--badge-neutral);
		color: var(--badge-neutral-text);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		white-space: nowrap;
		vertical-align: middle;
	}
	.ink {
		background: var(--badge-neutral);
		color: var(--badge-neutral-text);
	}
	.draft {
		background: var(--badge-draft);
		color: var(--badge-draft-text);
	}
	.accent,
	.published {
		background: var(--badge-published);
		color: var(--badge-published-text);
	}
	.soon {
		background: var(--badge-soon);
		color: var(--badge-soon-text);
	}
	.info {
		background: var(--badge-info);
		color: var(--badge-info-text);
	}
	.warning {
		background: var(--badge-warning);
		color: var(--badge-warning-text);
	}
	.error {
		background: var(--badge-error);
		color: var(--badge-error-text);
	}
</style>
