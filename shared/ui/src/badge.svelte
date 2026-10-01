<script lang="ts">
	import type { Snippet } from 'svelte';

	/**
	 * Status badge (claimed, revoked, draft, published, roles). Hairline border, caption size, square.
	 * FILLED (owner 2026-10-01), colours from tokens.css `--badge-*`: tone neutral · published (alias accent: live,
	 * signed) · draft (alias ink: claimed, source) · soon · info · warning · error. White text where it reaches 4.5:1,
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
		background: var(--badge-neutral-bg);
		color: var(--badge-neutral-fg);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		font-weight: var(--font-weight-bold);
		white-space: nowrap;
		vertical-align: middle;
	}
	.ink,
	.draft {
		background: var(--badge-draft-bg);
		color: var(--badge-draft-fg);
	}
	.accent,
	.published {
		background: var(--badge-published-bg);
		color: var(--badge-published-fg);
	}
	.soon {
		background: var(--badge-soon-bg);
		color: var(--badge-soon-fg);
	}
	.info {
		background: var(--badge-info-bg);
		color: var(--badge-info-fg);
	}
	.warning {
		background: var(--badge-warning-bg);
		color: var(--badge-warning-fg);
	}
	.error {
		background: var(--badge-error-bg);
		color: var(--badge-error-fg);
	}
</style>
