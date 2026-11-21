<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Icon, clickOutside } from '@scutoons/ui';

	/**
	 * A menu under its pill in the bar's top right (Views, History, Publish – as OSCT's and the deck editor's): the pill
	 * opens it, a press outside or Escape closes it. It lies over every panel (the bar's layer is above theirs).
	 */
	let { label, icon, about, testid, dot = false, wide = false, children }: { label: string; icon: string; about?: string; testid: string; dot?: boolean; wide?: boolean; children: Snippet<[close: () => void]> } = $props();

	let open = $state(false);
	const close = () => (open = false);
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && close()} />

<div class="bar-menu" {@attach clickOutside(close)}>
	<button type="button" class="pill" aria-expanded={open} aria-haspopup="menu" title={about ?? label} onclick={() => (open = !open)} data-testid={testid}>
		<Icon name={icon} size="1rem" />{label}{#if dot}<span class="dot" aria-hidden="true"></span>{/if}
	</button>
	{#if open}
		<div class="menu" class:wide data-testid="{testid}-menu">{@render children(close)}</div>
	{/if}
</div>

<style>
	.bar-menu {
		position: relative;
	}
	.pill {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		min-height: var(--control-height);
		padding: 0 0.85rem;
		border: var(--editor-edge) solid var(--editor-line);
		border-radius: 1.2rem;
		background: var(--editor-panel-ground);
		text-shadow: var(--editor-halo);
		color: var(--editor-text);
		white-space: nowrap;
	}
	.pill:hover,
	.pill[aria-expanded='true'] {
		background: var(--editor-hover);
	}
	.dot {
		width: 0.45rem;
		height: 0.45rem;
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
		box-shadow: 0 0.4rem 1.2rem rgb(0 0 0 / 0.4);
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
