<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Icon, clickOutside } from '@scutoons/ui';

	/**
	 * A menu under its icon at the bar's right end (Publish, the account): the icon – no text, its name the tooltip – opens
	 * it, a press outside or Escape closes it. It lies over every panel (the bar's layer is above theirs).
	 */
	let { label, icon, about, testid, dot = false, wide = false, children }: { label: string; icon: string; about?: string; testid: string; dot?: boolean; wide?: boolean; children: Snippet<[close: () => void]> } = $props();

	let open = $state(false);
	const close = () => (open = false);
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && close()} />

<div class="bar-menu" {@attach clickOutside(close)}>
	<button type="button" class="round" aria-expanded={open} aria-haspopup="menu" aria-label={label} title={about ?? label} onclick={() => (open = !open)} data-testid={testid}>
		<Icon name={icon} size="1rem" />{#if dot}<span class="dot" aria-hidden="true"></span>{/if}
	</button>
	{#if open}
		<div class="menu" class:wide data-testid="{testid}-menu">{@render children(close)}</div>
	{/if}
</div>

<style>
	.bar-menu {
		position: relative;
	}
	.round {
		position: relative;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: calc(var(--control-height) - 0.4rem);
		min-height: calc(var(--control-height) - 0.4rem);
		padding: 0;
		border: 0;
		border-radius: 50%;
		background: transparent;
		color: var(--editor-text);
	}
	.round:hover,
	.round[aria-expanded='true'] {
		background: var(--editor-hover);
	}
	.dot {
		position: absolute;
		top: 0.2rem;
		right: 0.2rem;
		width: 0.4rem;
		height: 0.4rem;
		border-radius: 50%;
		background: var(--editor-warn);
	}
	.menu {
		position: absolute;
		z-index: 60;
		top: calc(100% + 0.45rem);
		right: 0;
		display: flex;
		flex-direction: column;
		box-sizing: border-box;
		width: 19rem;
		max-height: calc(100dvh - var(--shell-top) - var(--status-room));
		padding: 0.3rem;
		overflow: auto;
		border: var(--editor-edge) solid var(--editor-line);
		border-radius: var(--editor-radius);
		background: var(--editor-surface);
		color: var(--editor-text);
	}
	.menu.wide {
		width: 26rem;
	}
	/* the lines of a menu (the Views list) */
	.menu :global(.line) {
		display: flex;
		align-items: center;
		justify-content: flex-start;
		gap: 0.6rem;
		min-height: var(--control-height);
		padding: 0 0.6rem;
		border: 0;
		border-radius: var(--editor-radius);
		background: transparent;
		color: var(--editor-text);
		text-align: left;
	}
	.menu :global(button.line:hover:not(:disabled)) {
		background: var(--editor-hover);
	}
	.menu :global(.head) {
		padding: 0.4rem 0.6rem 0.2rem;
		color: var(--editor-muted);
		font-size: var(--editor-label);
		letter-spacing: var(--editor-label-spacing);
		text-transform: uppercase;
	}
	.menu :global(.empty) {
		margin: 0;
		padding: 0.6rem;
		color: var(--editor-muted);
	}
	.menu :global(hr) {
		width: 100%;
		margin: 0.25rem 0;
		border: 0;
		border-top: var(--editor-edge) solid var(--editor-line);
	}
</style>
